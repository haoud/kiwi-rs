use core::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, collections::vec_deque::VecDeque, sync::Arc, vec::Vec};
use macros::{init, per_cpu};

use crate::{
    arch::{self, thread::KernelThreadStart},
    library::lock::spin::Spinlock,
    time::{duration::Duration, timer},
};

/// A type alias for a function that represents the entry point of a kernel
/// thread. The function takes no arguments and does not return, as it is
/// expected to either run indefinitely or terminate the thread by calling
/// [`arch::thread::exit()`].
pub type ThreadFn = fn() -> !;

/// Represents a task in the system, which is a unit of execution that can be
/// scheduled by the kernel. Each task is associated with a thread, which can
/// either be a kernel thread or a user thread.
#[derive(Debug)]
pub struct Task {
    /// The thread associated with this task. This is a low-level
    /// representation of the execution context of the task, designed
    /// to be architecture-specific.
    thread: Spinlock<arch::thread::Thread>,

    /// The current state of the task
    state: Spinlock<TaskState>,

    /// A unique identifier for the task, used for tracking and managing
    /// tasks within the scheduler.
    id: ThreadIdentifier,
}

impl Task {
    /// The time slice allocated to each task that a task is allowed to run
    /// continuously before the scheduler preempts it and switches to another
    /// task. Since we use a very simple round-robin scheduling algorithm, this
    /// value is fixed, but is unfair to tasks that do a lot of I/O since they
    /// will be put back into the end of the runnable queue and will have to
    /// wait for their next turn to run.
    pub const QUANTUM: Duration = Duration::from_millis(50);

    /// Creates a new task with the given thread.
    #[must_use]
    pub fn new(thread: arch::thread::Thread) -> Self {
        Self {
            thread: Spinlock::new(thread),
            state: Spinlock::new(TaskState::Created),
            id: ThreadIdentifier::generate(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &ThreadIdentifier {
        &self.id
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        log::info!("Dropping task {:?}", self.id.0);
    }
}

/// Represents the various states that a task can be in during its lifecycle.
/// This enum ensures that tasks transition through valid states and provides
/// a clear understanding of the task's current status within the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskState {
    /// The thread has been created but has not yet started running.
    Created,

    /// The thread is currently running on a CPU.
    Running,

    /// The thread is ready to run and is waiting to be scheduled on a CPU.
    Runnable,

    /// The thread is waiting for an event to occur before it can continue
    /// running. This could be waiting for I/O, a lock, or some other condition
    /// to be met.
    Blocked,

    /// The thread has finished executing and is no longer active.
    Exited,

    /// The thread has been terminated by the kernel due to a fatal error or
    /// other exceptional condition. This state is similar to [`Self::Exited`],
    /// but indicates that the thread did not complete its work successfully.
    Killed,

    /// The termination of the thread has been acknowledged by the kernel, and
    /// its resources are waiting to be cleaned up.
    Zombie,
}

impl TaskState {
    /// Updates the state of the task to indicate that it is no longer running
    /// and is now suspended and return a copy of the new state.
    #[allow(clippy::return_self_not_must_use)]
    pub fn suspend(&mut self) -> Self {
        match self {
            TaskState::Running => {
                // If the task is currently running and is now being
                // suspended from running: This is the expected behavior,
                // but we need to change the state to `Runnable` to indicate
                // that the task is no longer running and is now ready to be
                // scheduled again.
                *self = Self::Runnable;
            }
            TaskState::Runnable => {
                // Somehow, the task is being suspended while it is in the
                // runnable state. This is quite unexpected, and we print
                // a warning to indicate that something unusual is happening.
                log::warn!(
                    "Suspending a task that is in the runnable state (should be running): {:?}",
                    self
                );
            }
            TaskState::Blocked | TaskState::Exited | TaskState::Killed => {
                // If the task is currently blocked and is now being
                // suspended from running: This is the expected behavior,
                // and we don't need to do anything special here.
                //
                // If the task has exited or been killed and is now being
                // suspended from running: this is the expected behavior,
                // and we don't need to do anything special here. The task
                // is already in a terminal state, and suspending it doesn't
                // change that.
            }
            _ => unreachable!("Invalid state transition (suspended): {:?}", self),
        }
        *self
    }

    /// Updates the state of the task to indicate that it is now running.
    ///
    /// # Panics
    /// This function panics if the task is not in a state where it can
    /// transition to running, which is either `Created` or `Runnable`.
    /// If the task is in any other state, it is considered an invalid state
    /// transition, and is a serious logic bug in the kernel and we panic
    /// to avoid any data corruption or undefined behavior that could occur
    /// if we allowed the task to transition to running from an invalid state.
    pub fn running(&mut self) {
        match self {
            TaskState::Created | TaskState::Runnable => {
                *self = Self::Running;
            }
            _ => {
                unreachable!("Invalid state transition (from {:?} to Running)", self);
            }
        }
    }

    /// Returns true if the task is in a terminal state (i.e., it has exited,
    /// been killed, or is a zombie).
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        matches!(self, Self::Exited | Self::Killed | Self::Zombie)
    }

    /// Returns true if the task is in a state where it can be scheduled to run.
    #[must_use]
    pub fn is_runnable(&self) -> bool {
        matches!(self, Self::Runnable | Self::Running)
    }
}

/// A unique identifier for a thread, used for tracking and managing threads
/// within the scheduler.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadIdentifier(usize);

impl ThreadIdentifier {
    /// Generates a new unique thread identifier.
    #[must_use]
    pub fn generate() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<ThreadIdentifier> for usize {
    fn from(id: ThreadIdentifier) -> Self {
        id.0
    }
}

/// A global list of all tasks that currently exist in the system. This list
/// is used for tracking and managing tasks.
static TASKS: Spinlock<Vec<Arc<Task>>> = Spinlock::new(Vec::new());

/// A queue of tasks that are ready to run. Tasks are added to the back of the queue
/// when they become runnable, and are removed from the front of the queue when they
/// are scheduled to run.
static RUNNABLE: Spinlock<VecDeque<Arc<Task>>> = Spinlock::new(VecDeque::new());

/// The current task that is running on this CPU.
#[per_cpu]
static CURRENT_TASK: Spinlock<Option<Arc<Task>>> = Spinlock::new(None);

/// A flag indicating whether the current task needs to be rescheduled.
#[per_cpu]
static NEED_RESCHEDULE: Cell<bool> = Cell::new(false);

/// Sets up the scheduler
///
/// # Safety
/// This function should only be called from the kernel initialization code
/// once, before any threads are created or scheduled
#[init]
pub unsafe fn setup() {
    // Nothing to do here for now
}

/// Spawns a new kernel thread that will execute the given function.
pub fn spawn_fn(function: ThreadFn) {
    let task = Arc::new(Task::new(arch::thread::Thread::kernel(function)));
    TASKS.lock().push(Arc::clone(&task));
    RUNNABLE.lock().push_back(task);
}

/// Spawns a new kernel thread that will execute the given closure.
pub fn spawn<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    let task = Arc::new(Task::new(arch::thread::Thread::from_closure(
        kthread_runner,
        Box::new(f),
    )));
    TASKS.lock().push(Arc::clone(&task));
    RUNNABLE.lock().push_back(task);
}

/// Changes the state of the current task to the given state.
///
/// # Panics
/// This function panics if there is no current task, which indicates that the
/// scheduler is not running or has not been properly initialized.
pub fn change_current_task_state(state: TaskState) {
    CURRENT_TASK
        .local()
        .lock_irq_safe()
        .as_mut()
        .expect("No current task to change state for")
        .state
        .lock()
        .set(state);
}

/// Sets the flag indicating that the current task needs to be rescheduled.
/// This does not immediately cause a context switch, but rather sets a flag
/// that will be checked by the scheduler at the next opportunity, when the
/// current task traps.
pub fn set_need_reschedule(reschedule: bool) {
    NEED_RESCHEDULE.local().set(reschedule);
}

/// Returns true if the current task needs to be rescheduled, false otherwise.
#[must_use]
pub fn need_reschedule() -> bool {
    NEED_RESCHEDULE.local().get()
}

/// The heart of the scheduler.
///
/// This function runs in an infinite loop, continuously checking for runnable
/// tasks and executing them. If there are no runnable tasks, it will put the
/// CPU into a low-power state until an interrupt occurs, at which point it
/// will check for runnable tasks again.
pub fn run() -> ! {
    loop {
        let next = loop {
            if let Some(thread) = RUNNABLE.lock().pop_front() {
                thread.state.lock().running();
                break thread;
            }
            arch::irq::wait();
        };

        run_thread(&next);

        // Change the state of the task to indicate that it is no longer
        // running and is now suspended. If the task is still in a runnable
        // state, we add it back to the runnable queue so that it can be
        // scheduled again in the future.
        let state = {
            let mut state = next.state.lock();
            state.suspend();
            state.copy()
        };

        if state.is_runnable() {
            RUNNABLE.lock().push_back(next);
        } else if state.is_terminated() {
            TASKS.lock().retain(|t| !Arc::ptr_eq(t, &next));
        }
    }
}

/// Runs the given thread until it needs to be rescheduled. This function is
/// responsible for executing the thread's code and managing deferred work,
/// and ensures that the thread will be preempted after its time slice has
/// expired.
fn run_thread(thread: &Arc<Task>) {
    CURRENT_TASK.local().lock().replace(Arc::clone(thread));
    let _guard = timer::schedule_in(Task::QUANTUM, move |_| {
        set_need_reschedule(true);
    });

    while !need_reschedule() {
        arch::thread::execute(&mut thread.thread.lock());
        // TODO: Kernel work
    }

    CURRENT_TASK.local().lock().take();
    set_need_reschedule(false);
}

/// A helper function that serves as the entry point for a kernel thread to
/// execute a closure.
///
/// # Safety
/// The function assumes that the pointer passed to it is non-null, valid and
/// points to a proper `KernelThreadStart` structure allocated on the heap and
/// then leaked as a raw pointer. The function takes ownership of the pointer
/// and is responsible for deallocating it.
///
/// # Panics
/// This function panics if the given closure panics, which will cause the
/// kernel thread to terminate (in the future) and may make the kernel unstable.
unsafe extern "C" fn kthread_runner(start: *mut KernelThreadStart) -> ! {
    // SAFETY: The pointer is guaranteed to be valid and non-null, since
    // it was deconstructed from a Box during the creation of the kernel
    // thread because we call this function from assembly and we must
    // use a fixed calling convention, that does not support passing a Box
    // as an argument easily (since it is a fat pointer).
    let start = unsafe { Box::from_raw(start) };
    start.run();
    arch::thread::exit();
}
