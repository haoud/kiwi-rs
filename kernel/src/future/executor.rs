use crate::future::task::{self, Task};
use crate::{arch, future::user::thread_loop};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use crossbeam::queue::ArrayQueue;

/// The global executor instance, used to run all user-space tasks. This
/// executor replace the traditional term "scheduler" in the context of
/// user-space tasks.
///
/// The main advantage of having an executor instead of a scheduler is that
/// it allow to use cooperative multitasking inside the kernel with a single
/// kernel stack per core. This is possible because a future will compile to
/// a state machine that can be paused and resumed at specific points.
static EXECUTOR: spin::Once<Executor> = spin::Once::new();

/// The executor is responsible to run all user-space tasks. It behaves like
/// a simple First-In-First-Out (FIFO) cooperative scheduler.
///
/// # A cooperative scheduler for user-space tasks ?
/// You may wonder why we use a cooperative scheduler instead of a preemptive
/// scheduler for user-space tasks. This hasn't been done seriously since the
/// early `MacOS` versions that demonstrated that cooperative multitasking for
/// user tasks is not a good idea for a stable and modern operating system.
///
/// However, in our case, even if we use a cooperative executor, the user-space
/// are 100% preemptive ! This is because the cooperative part is only for the
/// kernel, and preemption is possible during every kernel trap (interrupt,
/// syscall...). Since userspace run with interrupts enabled (contrary to the
/// kernel), the kernel can preempt the user-space tasks at any time using a
/// cooperative approach in the kernel ;)
pub struct Executor<'a> {
    /// All tasks that was not running are stored in this map. The key is
    /// the task identifier and the value is the task itself. Tasks that
    /// are currently running are not stored in this map to avoid locking
    /// the map for every task poll, that would lead to an single-threaded
    /// executor...
    tasks: spin::Mutex<BTreeMap<task::Identifier, Task<'a>>>,

    /// The queue of tasks identifier that are ready to be executed.
    ready: ArrayQueue<task::Identifier>,
}

impl Executor<'_> {
    /// Create a new executor instance that can handle a maximum of
    /// `config::MAX_TASKS` tasks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: spin::Mutex::new(BTreeMap::new()),
            ready: ArrayQueue::new(usize::from(config::MAX_TASKS)),
        }
    }

    /// Return true if there are tasks ready to run.
    #[must_use]
    pub fn tasks_ready_to_run(&self) -> bool {
        !self.ready.is_empty()
    }

    /// Run the next task that is ready to run.
    ///
    /// # Panics
    /// Panics if this function encounters a duplicated task identifier. This
    /// should never happen because the task identifier is unique and encoded
    /// into a u64 that can handle up to 2^64 - 1 tasks and cannot be
    /// overflowed in a reasonable time.
    pub fn run_once(&self) {
        // Get the next task to run.
        if let Some(id) = self.ready.pop() {
            // If the task is not found in the map, this means that the
            // task has completed and was removed from the map. Therefore,
            // we can safely ignore it.
            let Some(mut task) = self.tasks.lock().remove(&id) else {
                log::trace!("Task {:?} already completed", usize::from(id));
                return;
            };

            match task.poll() {
                core::task::Poll::Ready(()) => {
                    // The task has completed. Therefore, we have nothing to
                    // do because the task was already removed from the map.
                    log::trace!("Task {:?} completed", usize::from(id));
                }
                core::task::Poll::Pending => {
                    // The task is not yet completed. Therefore, we must
                    // put it back in the map for the next run. The task
                    // identifier will be added to the ready queue by the
                    // task's waker when the task will be ready to run again.
                    assert!(self.tasks.lock().insert(id, task).is_none());
                }
            }
        }
    }

    /// Return a reference to the ready queue.
    #[must_use]
    pub const fn ready_queue(&self) -> &ArrayQueue<task::Identifier> {
        &self.ready
    }
}

impl Default for Executor<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Setup the global executor instance.
pub fn setup() {
    log::info!("Setting up the kernel executor");
    EXECUTOR.call_once(Executor::new);
}

/// Spawn a new future into the executor.
///
/// # Panics
/// Panics if the executor is not initialized (i.e. `setup` was not called).
pub fn spawn(thread: arch::thread::Thread) {
    let executor = EXECUTOR.get().expect("Executor not initialized");
    let task = Task::new(executor, Box::pin(thread_loop(thread)));
    let id = task.id();

    // Insert the task into the tasks map. If the task identifier already
    // exists in the map, this means that the task identifier is duplicated.
    // This should never happen because the task identifier is unique, and
    // is a serious bug that must be fixed.
    assert!(executor.tasks.lock().insert(id, task).is_none());
    executor.ready.push(id).expect("Ready queue full");
    log::trace!("Task {:?} spawned", usize::from(id));
}

/// Run the executor forever. If there are no tasks ready to run, the
/// executor will put the current core to a low-power state until a task
/// is ready to run.
///
/// # Panics
/// Panics if the executor is not initialized (i.e. `setup` was not called).
pub fn run() -> ! {
    let executor = EXECUTOR.get().expect("Executor not initialized");

    loop {
        executor.run_once();
        while !executor.tasks_ready_to_run() {
            arch::cpu::relax();
        }
    }
}
