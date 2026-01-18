use crate::config;
use crate::future::task::{self, Task};
use crate::{arch, future::user::thread_loop};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
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

/// The global poll generation counter. This is used to track the number
/// of times the executor has polled tasks. This can be useful if a task
/// wants to know if it has yielded since the last time it checked.
static POLL_GENERATION: ExecutorGeneration = ExecutorGeneration::new();

/// The identifier of the currently running task on this core. This is
/// used to identify the task that is currently running on this core.
/// This is useful for syscalls that need to know the current task
/// identifier. If no task is running, this will be `None`.
static CURRENT_TASK_ID: spin::Mutex<Option<task::Identifier>> = spin::Mutex::new(None);

/// The executor is responsible to run all user-space tasks.
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

    /// The queue of tasks that are ready to be executed. Tasks are sorted
    /// by their virtual runtime: The task with the lowest virtual runtime is
    /// at the front of the queue and will be executed next.
    ready_queue: spin::Mutex<BTreeMap<u64, task::Identifier>>,

    /// The queue of tasks identifier that are ready to be executed, but was
    /// not yet inserted in the `ready_queue` map. This is used to avoid
    /// locking the `ready_queue` map for every task wake-up.
    ready_ids: ArrayQueue<task::Identifier>,
}

impl Executor<'_> {
    /// Create a new executor instance that can handle a maximum of
    /// `config::MAX_TASKS` tasks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: spin::Mutex::new(BTreeMap::new()),
            ready_queue: spin::Mutex::new(BTreeMap::new()),
            ready_ids: ArrayQueue::new(usize::from(config::MAX_TASKS)),
        }
    }

    /// Run the next task that is ready to run. If there are no tasks ready
    /// to run, this function does nothing.
    ///
    /// # Panics
    /// Panics if this function encounters a duplicated task identifier. This
    /// should never happen because the task identifier is unique and encoded
    /// into a u64 that can handle up to 2^64 - 1 tasks and cannot be
    /// overflowed in a reasonable time.
    pub fn run_once(&self) {
        self.process_ready_ids();

        // Get the next task to run.
        if let Some((_, id)) = self.ready_queue.lock().pop_first() {
            // If the task is not found in the map, this means that the
            // task has completed and was removed from the map. Therefore,
            // we can safely ignore it.
            let Some(mut task) = self.tasks.lock().remove(&id) else {
                log::trace!("Task {:?} already completed", usize::from(id));
                return;
            };

            // Set the current task ID to the task that is being run now.
            set_current_task_id(id);

            // TODO: Measure the time spent in the task for accounting purposes

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

            // Clear the current task ID because no task is running now and
            // increment the poll generation to indicate that we have polled
            // one more task.
            POLL_GENERATION.increment();
            clear_current_task_id();
        }
    }

    /// Process all the ready task identifiers and insert them into the
    /// ready queue, sorted by their virtual runtime.
    fn process_ready_ids(&self) {
        let mut ready_queue = self.ready_queue.lock();
        let lowest_vruntime = ready_queue.keys().next().copied().unwrap_or(0);
        let mut tasks = self.tasks.lock();

        while let Some(id) = self.ready_ids.pop() {
            if let Some(task) = tasks.get_mut(&id) {
                // Insert the task into the ready queue, using its virtual
                // runtime as the key. We ensure that the virtual runtime
                // is at least the lowest virtual runtime of all ready
                // tasks to avoid the case where a task has slept for a
                // long time and has a very low virtual runtime that would
                // starve all other tasks.
                // TODO: Since the task has slept for a long time, maybe we
                // should give it a small boost ? This may help interactive
                // tasks to be more responsive.
                let mut vruntime = task.vruntime().max(lowest_vruntime);

                // Increment the vruntime slightly if there is already a task
                // with the same vruntime in the ready queue to avoid
                // duplicated keys in the BTreeMap that would overwrite the
                // previous task stored with the same vruntime. This should not
                // happen often, and even if it does, the increment is very
                // small (1 nanosecond) and should not impact the scheduling
                // fairness.
                while ready_queue.contains_key(&vruntime) {
                    vruntime += 1;
                }

                ready_queue.insert(vruntime, id);
                task.set_vruntime(vruntime);
            } else {
                log::warn!("Task #{:?} not found in tasks map", usize::from(id));
            }
        }
    }

    /// Return true if there are tasks ready to run.
    #[must_use]
    pub fn tasks_ready_to_run(&self) -> bool {
        !self.ready_queue.lock().is_empty() || !self.ready_ids.is_empty()
    }

    /// Return a reference to the ready queue.
    #[must_use]
    pub const fn ready_ids(&self) -> &ArrayQueue<task::Identifier> {
        &self.ready_ids
    }
}

impl Default for Executor<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// An opaque generation counter for the executor. This is used to track the
/// number of times the executor has polled tasks. This can be useful if a task
/// wants to know if it has yielded since the last time it checked, and is
/// heavily used in the `thread_loop` future.
#[derive(Debug)]
pub struct ExecutorGeneration(AtomicU64);

impl ExecutorGeneration {
    /// Create a new generation counter initialized to 0.
    #[must_use]
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Increment the generation counter.
    fn increment(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the current generation value.
    fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl From<u64> for ExecutorGeneration {
    fn from(value: u64) -> Self {
        Self(AtomicU64::new(value))
    }
}

impl PartialEq for ExecutorGeneration {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for ExecutorGeneration {}

/// Setup the global executor instance.
pub fn setup() {
    log::info!("Setting up the kernel executor");
    EXECUTOR.call_once(Executor::new);
}

/// Return the identifier of the currently running task on this core. If no
/// task is running, this will return `None`.
pub fn current_task_id() -> Option<task::Identifier> {
    *CURRENT_TASK_ID.lock()
}

/// Spawn a new future into the executor.
///
/// # Panics
/// Panics if the executor is not initialized (i.e. `setup` was not called).
pub fn spawn(thread: arch::thread::Thread) {
    let executor = EXECUTOR.get().expect("Executor not initialized");

    // Compute the virtual runtime of the new task. We take the lowest
    // virtual runtime of all ready tasks to ensure that the new task does
    // not starve other tasks since they will all have a higher virtual
    // runtime. If there are no ready tasks, we set the virtual runtime to 0.
    let vruntime = executor
        .ready_queue
        .lock()
        .keys()
        .next()
        .copied()
        .unwrap_or(0);

    let task = Task::new(executor, Box::pin(thread_loop(thread)), vruntime);
    let id = task.id();

    // Insert the task into the tasks map. If the task identifier already
    // exists in the map, this means that the task identifier is duplicated.
    // This should never happen because the task identifier is unique, and
    // is a serious bug that must be fixed.
    assert!(executor.tasks.lock().insert(id, task).is_none());
    executor.ready_ids.push(id).expect("Ready queue full");
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

/// Return the current poll generation of the executor.
#[must_use]
pub fn poll_generation() -> ExecutorGeneration {
    ExecutorGeneration::from(POLL_GENERATION.get())
}

/// Return true if the executor has polled a task since the given generation,
/// indicating that the task has yielded, and false otherwise.
#[must_use]
pub fn has_yielded(since: &ExecutorGeneration) -> bool {
    POLL_GENERATION.get() != since.get()
}

/// Set the identifier of the currently running task on this core.
fn set_current_task_id(id: task::Identifier) {
    *CURRENT_TASK_ID.lock() = Some(id);
}

/// Clear the identifier of the currently running task on this core.
fn clear_current_task_id() {
    *CURRENT_TASK_ID.lock() = None;
}
