use core::cell::Cell;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use macros::{init, per_cpu};

use crate::{
    arch::{self, irq::State, thread::KernelThreadStart},
    future::{self, executor::ThreadAction},
    library::lock::spin::Spinlock,
    scheduler::task::{Task, TaskState, ThreadFn},
    time::{duration::Duration, instant::Instant},
};

pub mod load;
pub mod policy;
pub mod task;

/// A trait that defines the behavior of a scheduler policy. A scheduler policy
/// is responsible for determining which task to run next and how to manage the
/// state of tasks in the system. The policy is set during kernel initialization
/// and is used by the scheduler to make scheduling decisions.
///
/// The policy is protected by a global lock shared by all CPUs, so it never
/// runs a task itself: it only makes decisions, and the generic scheduler code
/// runs the task without holding the policy lock. Every time the running task
/// traps back into the scheduler (at least once per timer tick), the policy is
/// informed of the time consumed by the task through [`Policy::task_tick`] and
/// decides whether the task should be preempted. This allows to implement
/// many kinds of policies with the same interface:
/// - A round-robin policy preempts the task once its time slice is consumed.
/// - A priority-based policy preempts the task as soon as a task with a higher
///   priority has become runnable.
/// - A fair policy (e.g. CFS or EEVDF) updates the virtual runtime of the task
///   and preempts it when another task has an earlier virtual deadline.
pub trait Policy {
    /// Called when a task becomes runnable (newly created, woken up or
    /// preempted). The task must be returned later by [`Policy::next_task`].
    fn task_runnable(&mut self, task: Arc<Task>);

    /// Called when a task is blocked and can no longer be run until it
    /// becomes runnable again.
    fn task_blocked(&mut self, task: Arc<Task>);

    /// Called when a task has terminated and will never run again.
    fn task_terminated(&mut self, task: Arc<Task>);

    /// Selects the next task to run on the current CPU, removing it from the
    /// runnable tasks of the policy, or returns `None` if there is no task to
    /// run.
    fn next_task(&mut self) -> Option<Arc<Task>>;

    /// Called every time the running task traps back into the scheduler, with
    /// information about the time it has consumed. Returns whether the task
    /// should keep running or be preempted.
    ///
    /// This function is called very frequently, so it should be fast.
    fn task_tick(&mut self, task: &Arc<Task>, tick: &TaskSlice) -> Preemption;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSlice {
    /// The current time.
    now: Instant,

    /// The time when the task was selected to run on this CPU.
    start: Instant,

    /// The time consumed by the task since the previous tick, or since it
    /// started to run if this is the first tick. This is useful for policies
    /// that account the runtime of tasks (e.g. virtual runtime).
    delta: Duration,

    /// The time consumed by the task since it was selected to run on this
    /// CPU. This is useful for policies that use time slices.
    elapsed: Duration,
}

impl TaskSlice {
    #[must_use]
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            now,
            start: now,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
        }
    }

    pub fn update(&mut self, now: Instant) {
        let delta = now.duration_since(self.now);
        let elapsed = now.duration_since(self.start);
        self.now = now;
        self.delta = delta;
        self.elapsed = elapsed;
    }

    /// Returns the time when the task was interrupted and returned to the
    /// scheduler, which is more or less the current time.
    #[must_use]
    pub fn now(&self) -> Instant {
        self.now
    }

    /// Returns the time when the task was selected to run on this CPU.
    #[must_use]
    pub fn start(&self) -> Instant {
        self.start
    }

    /// Returns the time consumed by the task since the previous tick, or since
    /// it started to run if this is the first tick.
    #[must_use]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Returns the time consumed by the task since it was selected to run on this
    /// CPU.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Default for TaskSlice {
    fn default() -> Self {
        Self::new()
    }
}

/// The decision made by the policy each time the running task traps back into
/// the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preemption {
    /// The task can continue to run on this CPU.
    Continue,

    /// The task must be preempted: it will be put back into the runnable
    /// tasks of the policy, and another task will be selected.
    Preempt,
}

/// A global list of all tasks that currently exist in the system. This list
/// is used for tracking and managing tasks.
static TASKS: Spinlock<Vec<Arc<Task>>> = Spinlock::new(Vec::new());

/// A global scheduler policy that determines how tasks are scheduled and
/// executed. This policy is set during kernel initialization and is used by
/// the scheduler to determine which task to run next.
static POLICY: Spinlock<Option<Box<dyn Policy + Send + Sync>>> = Spinlock::new(None);

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
    POLICY
        .lock()
        .replace(Box::new(policy::rr::RoundRobin::new()));
}

/// Spawns a new kernel thread that will execute the given function.
///
/// # Panics
/// This function panics if there is no scheduler policy set, which indicates
/// that the scheduler has not been properly initialized or configured.
pub fn spawn_fn(function: ThreadFn) {
    let task = Arc::new(Task::new(arch::thread::Thread::kernel(function)));
    TASKS.lock().push(Arc::clone(&task));
    POLICY
        .lock()
        .as_mut()
        .expect("No scheduler policy set")
        .task_runnable(task);
}

/// Spawns a new kernel thread that will execute the given closure.
///
/// # Panics
/// This function panics if there is no scheduler policy set, which indicates
/// that the scheduler has not been properly initialized or configured.
pub fn spawn<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    let task = Arc::new(Task::new(arch::thread::Thread::from_closure(
        kthread_runner,
        Box::new(f),
    )));
    TASKS.lock().push(Arc::clone(&task));
    POLICY
        .lock()
        .as_mut()
        .expect("No scheduler policy set")
        .task_runnable(task);
}

/// Returns the number of tasks that currently exist in the system.
#[must_use]
pub fn task_count() -> usize {
    TASKS.lock().len()
}

/// Returns the identifier and the state of every task that currently exists
/// in the system.
#[must_use]
pub fn tasks() -> Vec<(usize, TaskState)> {
    TASKS
        .lock()
        .iter()
        .map(|task| (usize::from(task.id().clone()), *task.state().lock()))
        .collect()
}

/// Changes the state of the current task to the given state. Be careful
/// when using this function, as changing the state of the current task can
/// lead to unexpected behavior if not done correctly...
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
        .state()
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

/// Schedules the next task to run.
///
/// This function sets the `NEED_RESCHEDULE` flag to true and then waits for an
/// interrupt to occur, which will trigger the rescheduling of the current
/// task.
/// TODO: Implement an immediate rescheduling mechanism that does not require
/// waiting for an interrupt.
///
/// # Panics
/// This function panics if interrupts are not enabled, which indicates that
/// the scheduler cannot safely perform a context switch at this time because
/// the caller is in a critical section or has disabled interrupts for some
/// other reason that can make a context switch unsafe.
pub fn schedule() {
    assert!(arch::irq::enabled() == State::Enabled);
    set_need_reschedule(true);
    load::idle_wait();
}

/// Returns the next runnable task from the scheduler policy. If there are no
/// runnable tasks, this function will block until one becomes available. The
/// returned task will have its state changed to `Running` before being
/// returned to the caller.
///
/// While there is no runnable task, the executor of the current CPU is run so
/// that kernel futures can make progress even if no thread is running on this
/// CPU. If supported by the architecture, this function will then put the CPU
/// into a low-power state until an interrupt occurs in order to save power and
/// reduce heat generation, unless the executor still has ready tasks.
///
/// # Panics
/// This function panics if there is no scheduler policy set, which indicates
/// that the scheduler has not been properly initialized or configured.
pub fn get_runnable_task() -> Arc<Task> {
    loop {
        if let Some(thread) = POLICY
            .lock()
            .as_mut()
            .expect("No scheduler policy set")
            .next_task()
        {
            thread.state().lock().running();
            break thread;
        }

        // There is no thread to run on this CPU: Use this time to run the
        // kernel futures. There is no current thread, so the returned action
        // is meaningless and is ignored.
        let _ = future::executor::run();
        if !future::executor::has_ready_tasks() {
            load::idle_wait();
        }
    }
}

/// Runs the given thread until it needs to be rescheduled. This function is
/// responsible for executing the thread's code and managing deferred work,
/// and lets the scheduler policy decide when the thread must be preempted.
///
/// The policy lock is never held while the thread is running, but only for
/// the short time needed by the policy to make its decision after each trap.
/// Otherwise, the lock would be held for the whole execution of the thread,
/// preventing the other CPUs from scheduling their own threads (and running
/// their executor), and deadlocking if the thread tries to spawn a thread.
///
/// Since each CPU receives a periodic timer interrupt, the thread traps at
/// least once per timer tick, so the policy can preempt it with the precision
/// of a tick, on the CPU that is actually running it.
/// TODO: When a task with a higher priority becomes runnable, the policy can
/// only preempt the running task at its next trap (at most one tick later).
/// An IPI could be sent to the CPU to preempt the task immediately.
fn run_thread(thread: &Arc<Task>) {
    let mut slice = TaskSlice::new();

    CURRENT_TASK.local().lock().replace(Arc::clone(thread));
    arch::irq::without(|| {
        while !need_reschedule() {
            arch::thread::execute(&mut thread.thread().lock());
            slice.update(Instant::now());

            match future::executor::run() {
                ThreadAction::Continue => (),
                ThreadAction::Sleep => {
                    thread.state().lock().block();
                    break;
                }
                ThreadAction::Abort => panic!("A future panicked inside the scheduler"),
            }

            // Inform the policy of the time consumed by the thread, and let
            // it decide whether the thread should be preempted.
            let preemption = POLICY
                .lock()
                .as_mut()
                .expect("No scheduler policy set")
                .task_tick(thread, &slice);
            if preemption == Preemption::Preempt {
                break;
            }
        }
        set_need_reschedule(false);
    });
    CURRENT_TASK.local().lock().take();
}

/// The heart of the scheduler.
///
/// This function runs in an infinite loop, continuously checking for runnable
/// tasks and executing them. If there are no runnable tasks, it will put the
/// CPU into a low-power state until an interrupt occurs, at which point it
/// will check for runnable tasks again.
///
/// # Panics
/// This function panics if there is no scheduler policy set, which indicates
/// that the scheduler has not been properly initialized or configured.
pub fn run() -> ! {
    assert!(arch::irq::enabled() == State::Enabled);

    loop {
        let next = get_runnable_task();
        run_thread(&next);

        // Change the state of the task to indicate that it is no longer
        // running and is now suspended and inform the scheduler policy
        // of the new state of the task.
        let state = {
            let mut state = next.state().lock();
            state.suspend()
        };

        if state.is_runnable() {
            POLICY
                .lock()
                .as_mut()
                .expect("No scheduler policy set")
                .task_runnable(next);
        } else if state.is_blocked() {
            POLICY
                .lock()
                .as_mut()
                .expect("No scheduler policy set")
                .task_blocked(next);
        } else if state.is_terminated() {
            TASKS.lock().retain(|t| !Arc::ptr_eq(t, &next));
            POLICY
                .lock()
                .as_mut()
                .expect("No scheduler policy set")
                .task_terminated(next);
        }
    }
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
