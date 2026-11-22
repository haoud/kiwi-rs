use core::cell::Cell;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use macros::{init, per_cpu};

use crate::{
    arch::{self, irq::State, thread::KernelThreadStart},
    library::lock::spin::Spinlock,
    scheduler::task::{Task, TaskState, ThreadFn},
};

pub mod policy;
pub mod task;

/// A trait that defines the behavior of a scheduler policy. A scheduler policy
/// is responsible for determining which task to run next and how to manage the
/// state of tasks in the system. The policy is set during kernel initialization
/// and is used by the scheduler to make scheduling decisions.
pub trait Policy {
    fn task_runnable(&mut self, task: Arc<Task>);
    fn task_blocked(&mut self, task: Arc<Task>);
    fn task_terminated(&mut self, task: Arc<Task>);
    fn next_task(&mut self) -> Option<Arc<Task>>;
    fn run_task(&mut self, task: &Arc<Task>);
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
    arch::irq::wait();
}

/// Returns the next runnable task from the scheduler policy. If there are no
/// runnable tasks, this function will block until one becomes available. The
/// returned task will have its state changed to `Running` before being
/// returned to the caller.
///
/// If supported by the architecture, this function will put the CPU into a
/// low-power state until an interrupt occurs in order to save power and reduce
///  heat generation.
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
        arch::irq::wait();
    }
}

/// Runs the given thread until it needs to be rescheduled. This function is
/// responsible for executing the thread's code and managing deferred work,
/// and ensures that the thread will be preempted after its time slice has
/// expired.
fn run_thread(thread: &Arc<Task>) {
    CURRENT_TASK.local().lock().replace(Arc::clone(thread));
    arch::irq::without(|| {
        POLICY
            .lock()
            .as_mut()
            .expect("No scheduler policy set")
            .run_task(thread);
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
