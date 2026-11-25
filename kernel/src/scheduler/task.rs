use core::sync::atomic::AtomicUsize;

use crate::{arch, library::lock::spin::Spinlock};

/// A type alias for a function that represents the entry point of a kernel
/// thread. The function takes no arguments and does not return, as it is
/// expected to either run indefinitely or terminate the thread by calling
/// [`arch::thread::exit()`].
pub type ThreadFn = fn() -> !;

/// A unique identifier for a thread, used for tracking and managing threads
/// within the scheduler.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskIdentifier(usize);

impl TaskIdentifier {
    /// Generates a new unique thread identifier.
    #[must_use]
    pub fn generate() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
    }
}

impl From<TaskIdentifier> for usize {
    fn from(id: TaskIdentifier) -> Self {
        id.0
    }
}

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
    id: TaskIdentifier,
}

impl Task {
    /// Creates a new task with the given thread.
    #[must_use]
    pub fn new(thread: arch::thread::Thread) -> Self {
        Self {
            thread: Spinlock::new(thread),
            state: Spinlock::new(TaskState::Created),
            id: TaskIdentifier::generate(),
        }
    }

    /// Returns a reference to the thread associated with this task, protected
    /// by a spinlock to ensure safe concurrent access.
    #[must_use]
    pub fn thread(&self) -> &Spinlock<arch::thread::Thread> {
        &self.thread
    }

    /// Returns a reference to the state of this task, protected by a spinlock
    /// to ensure safe concurrent access.
    #[must_use]
    pub fn state(&self) -> &Spinlock<TaskState> {
        &self.state
    }

    /// Returns a reference to the unique identifier of this task.
    #[must_use]
    pub fn id(&self) -> &TaskIdentifier {
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
    /// Updates the state of the task to indicate that it is now blocked and
    /// returns a copy of the new state.
    #[allow(clippy::return_self_not_must_use)]
    pub fn block(&mut self) -> Self {
        match self {
            TaskState::Running => {
                *self = Self::Blocked;
            }
            TaskState::Blocked => {}
            _ => unreachable!("Invalid state transition (blocked): {:?}", self),
        }
        *self
    }

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

    /// Returns true if the task is in a state where it is waiting for an event
    /// to occur before it can continue running.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }
}
