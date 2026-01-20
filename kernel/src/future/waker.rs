use super::task::{self};
use alloc::sync::Arc;
use crossbeam::queue::ArrayQueue;

/// A waker that can wake up a task.
#[derive(Debug)]
pub struct Waker {
    /// The que to push the task identifier to when waking
    /// up the task.
    queue: Arc<ArrayQueue<task::Identifier>>,

    /// The identifier of the task to wake up.
    pub id: task::Identifier,
}

impl Waker {
    /// Create a new waker.
    #[must_use]
    pub fn new(queue: Arc<ArrayQueue<task::Identifier>>, id: task::Identifier) -> Self {
        Waker { queue, id }
    }
}

impl alloc::task::Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.queue.push(self.id).expect("Queue is full");
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.queue.push(self.id).expect("Queue is full");
    }
}
