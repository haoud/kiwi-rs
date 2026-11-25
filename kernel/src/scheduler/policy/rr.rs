//! A simple round-robin scheduler policy implementation. This policy is very
//! simple and does not take into account any task priorities or other factors.
//! Useful for testing and debugging, but not suitable for production use.
use alloc::{collections::VecDeque, sync::Arc};

use crate::{
    scheduler::{Policy, Preemption, Task, TaskSlice},
    time::duration::Duration,
};

/// A simple round-robin scheduler policy implementation that maintains a queue
/// of runnable tasks and schedules them in a first-come, first-served manner.
///
/// Each task is given a fixed time slice (defined by `QUANTUM`) to run before
/// being preempted and placed back at the end of the queue. If a task yields
/// or blocks before its time slice expires, it is removed from the queue and
/// will not be scheduled again until it becomes runnable again.
///
/// The consequence of this simple scheduling policy is that tasks that do a
/// lot of I/O will be HEAVILY penalized, as they will be put back into the end
/// of the runnable queue even if they have been waiting for a long time or
/// have a lot of remaining quantum to run.
pub struct RoundRobin {
    runnable: VecDeque<Arc<Task>>,
}

impl RoundRobin {
    /// The time slice allocated to each task that a task is allowed to run
    /// continuously before the scheduler preempts it and switches to another
    /// task. Since we use a very simple round-robin scheduling algorithm, this
    /// value is fixed, but is unfair to tasks that do a lot of I/O since they
    /// will be put back into the end of the runnable queue and will have to
    /// wait for their next turn to run.
    pub const QUANTUM: Duration = Duration::from_millis(50);

    /// Creates a new instance of the round-robin scheduler policy with an empty
    /// runnable queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            runnable: VecDeque::new(),
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl Policy for RoundRobin {
    /// Adds a task to the runnable queue, indicating that it is ready to be
    /// scheduled and executed by the scheduler.
    fn task_runnable(&mut self, task: Arc<Task>) {
        self.runnable.push_back(task);
    }

    /// Removes a task from the runnable queue, indicating that it is no longer
    /// ready to be scheduled and executed by the scheduler.
    fn task_blocked(&mut self, task: Arc<Task>) {
        if let Some(pos) = self.runnable.iter().position(|t| Arc::ptr_eq(t, &task)) {
            self.runnable.remove(pos);
        }
    }

    /// Removes a task from the runnable queue, indicating that it has completed
    /// its execution and is no longer needed by the scheduler.
    fn task_terminated(&mut self, task: Arc<Task>) {
        if let Some(pos) = self.runnable.iter().position(|t| Arc::ptr_eq(t, &task)) {
            self.runnable.remove(pos);
        }
    }

    /// Returns the next task to be scheduled and executed by the scheduler. If
    /// there are no runnable tasks, this function returns `None`, indicating
    /// that no task is available for execution.
    fn next_task(&mut self) -> Option<Arc<Task>> {
        self.runnable.pop_front()
    }

    /// Preempts the task once it has consumed its fixed time slice (defined
    /// by `QUANTUM`), regardless of the other runnable tasks.
    fn task_tick(&mut self, _task: &Arc<Task>, tick: &TaskSlice) -> Preemption {
        if tick.elapsed >= RoundRobin::QUANTUM {
            Preemption::Preempt
        } else {
            Preemption::Continue
        }
    }
}
