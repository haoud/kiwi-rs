use core::cell::LazyCell;

use alloc::{boxed::Box, collections::BTreeMap, collections::VecDeque, sync::Arc};
use macros::per_cpu;

use crate::{
    future::{self, task::LocalTask},
    library::lock::spin::Spinlock,
};

/// A per-CPU executor that drives kernel futures to completion. Each CPU has
/// its own executor, and a future spawned on a CPU will always be polled on
/// that same CPU, which avoids any synchronization between CPUs when polling
/// futures and keeps the data used by a future hot in the CPU caches.
///
/// However, a future can be woken up from any CPU (or from an interrupt
/// handler) since the wakers only push the identifier of the task into the
/// ready queue of the executor that owns the task.
pub struct LocalExecutor {
    /// The identifiers of the tasks that are ready to be polled. A task is
    /// pushed into this queue when it is spawned and every time it is woken
    /// up by its waker.
    pub ready_kernel_tasks: Spinlock<VecDeque<future::task::Identifier>>,

    /// All the tasks owned by this executor that have not completed yet. A
    /// task is temporarily removed from this map while it is being polled, so
    /// that the lock is not held during the poll. This allows a future to
    /// spawn new tasks on the same executor without deadlocking.
    tasks: Spinlock<BTreeMap<future::task::Identifier, LocalTask>>,
}

impl LocalExecutor {
    /// Creates a new executor without any task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ready_kernel_tasks: Spinlock::new(VecDeque::new()),
            tasks: Spinlock::new(BTreeMap::new()),
        }
    }

    /// Adds a new task to this executor and immediately marks it as ready to
    /// be polled.
    pub fn spawn(self: &Arc<Self>, future: impl Future<Output = ()> + Send + 'static) {
        let task = LocalTask::new(Arc::clone(self), Box::pin(future));
        let id = task.id();

        self.tasks.lock_irq_safe().insert(id, task);
        self.ready_kernel_tasks.lock_irq_safe().push_back(id);
    }

    /// Returns true if at least one task is waiting to be polled.
    #[must_use]
    pub fn has_ready_tasks(&self) -> bool {
        !self.ready_kernel_tasks.lock_irq_safe().is_empty()
    }

    /// Polls all the tasks that are ready at the time this function is called.
    ///
    /// Tasks that are woken up during this run (including a task that wakes
    /// itself up) will only be polled during the next run. This bounds the
    /// time spent in this function and prevents a task that always wakes
    /// itself up from starving the threads of this CPU.
    pub fn run(&self) -> ThreadAction {
        let pending = self.ready_kernel_tasks.lock_irq_safe().len();

        for _ in 0..pending {
            let Some(id) = self.ready_kernel_tasks.lock_irq_safe().pop_front() else {
                break;
            };

            // The task may have already completed if it was woken up multiple
            // times before being polled: in that case, the identifier is
            // simply ignored.
            let Some(mut task) = self.tasks.lock_irq_safe().remove(&id) else {
                continue;
            };

            // Poll the task without holding any lock, and put it back into
            // the task list if it has not completed yet. If it has completed,
            // the task is dropped here, along with its future.
            if task.poll().is_pending() {
                self.tasks.lock_irq_safe().insert(id, task);
            }
        }

        ThreadAction::Continue
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// The action that the scheduler should take for the current thread after
/// the executor has been run.
pub enum ThreadAction {
    /// The current thread can continue its execution.
    Continue,

    /// The current thread must be blocked until an event wakes it up. This is
    /// not produced yet, but will be used when the kernel work of a thread
    /// (e.g. a syscall) is executed as a future: the thread will sleep until
    /// the future completes.
    Sleep,

    /// A fatal error occurred while executing a future. Since the kernel is
    /// compiled with `panic = "abort"`, this is not produced yet either.
    Abort,
}

/// The executor of the current CPU. It is lazily created the first time it is
/// used, because per-CPU variables are initialized before the heap is set up.
#[per_cpu]
static EXECUTOR: LazyCell<Arc<LocalExecutor>> = LazyCell::new(|| Arc::new(LocalExecutor::new()));

/// Spawns a new kernel task on the executor of the current CPU. The task will
/// always be polled on this CPU, the next time the executor runs.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    EXECUTOR.local().spawn(future);
}

/// Returns true if the executor of the current CPU has tasks ready to be
/// polled.
#[must_use]
pub fn has_ready_tasks() -> bool {
    EXECUTOR.local().has_ready_tasks()
}

/// Runs the executor of the current CPU, polling all tasks that are ready to
/// make progress. See [`LocalExecutor::run`] for more details.
///
/// The per-CPU guard is dropped before polling the tasks, so futures are
/// running with interrupts enabled and does not hold any lock on the executor.
#[must_use]
pub fn run() -> ThreadAction {
    let executor = Arc::clone(&EXECUTOR.local());
    executor.run()
}
