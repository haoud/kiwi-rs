use intrusive_collections::{intrusive_adapter, LinkedListLink};

/// The kernel task list. Because the kernel does not handle the memory management,
/// it cannot dynamically allocate memory for tasks. Instead, the kernel uses a
/// statically allocated array of tasks that will be used throughout the kernel's
/// lifetime.
///
/// Althrough it increases the kernel's memory footprint, it simplifies the kernel
/// code and even allows use to use intrusve collections to manage the tasks, which
/// would be more difficult with dynamic memory allocation and will involve more
/// runtime overhead (e.g. `Arc` and `Rc` everywhere).
///
/// For embedded systems with limited resources, this is a common approach to
/// reduce the [`config::MAX_TASKS`] to a smaller number than default to save
/// memory. This is even better if the maximal number of tasks is known at compile
/// time.
pub static TASK_LIST: [spin::Mutex<Task>; config::MAX_TASKS as usize] =
    [DEFAULT_TASK; config::MAX_TASKS as usize];

#[doc(hidden)]
#[allow(clippy::declare_interior_mutable_const)]
const DEFAULT_TASK: spin::Mutex<Task> = spin::Mutex::new(Task::new());

intrusive_adapter!(TaskAdapter<'a> = &'a Task<'a>: Task { link: LinkedListLink });

#[derive(Debug)]
pub struct Task<'a> {
    /// The task's pager. This is the task that will receive a message when a
    /// page fault occurs. This is used to implement demand paging.
    pager: Option<&'a Task<'a>>,

    /// The state of the task. This is used to determine the task's current state
    /// and to schedule the task accordingly.
    state: State,

    /// An adapter for the intrusive linked list. This allows the task to be
    /// part of an intrusive linked list that can be used with an safe API.
    link: LinkedListLink,
}

unsafe impl<'a> Send for Task<'a> {}

impl Task<'_> {
    /// Create a new task. This is a const function that should only be used to
    /// initialize the task list.
    #[must_use]
    const fn new() -> Self {
        Self {
            link: LinkedListLink::new(),
            state: State::Unused,
            pager: None,
        }
    }

    /// The task's pager.
    #[must_use]
    pub fn pager(&self) -> Option<&Task> {
        self.pager
    }

    /// The task's state.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }
}

/// The task's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The task is not used and can be reclaimed to execute a task
    /// from the userspace.
    Unused,

    /// The task is being created and is not yet ready to run.
    Created,

    /// The task is ready to run and is waiting to be scheduled.
    Ready,

    /// The task is currently running on the CPU.
    Running,

    /// The task is blocked and is waiting for an event to occur.
    Blocked,

    /// The task is dead and will be removed from the task list, but its
    /// resources will not be reclaimed until the task is joined by another
    /// task.
    Dead,
}

/// Get an unused task from the task list. This is a helper function that will
/// return the first task that is not used. If no task is available, it will
/// return `None`.
pub fn get_unused_task() -> Option<&'static spin::Mutex<Task<'static>>> {
    TASK_LIST
        .iter()
        .find(|task| task.lock().state() == State::Unused)
}
