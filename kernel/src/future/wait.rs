use alloc::sync::Arc;
use core::{
    pin::Pin,
    task::{Context, Poll, Waker},
};
use crossbeam::queue::SegQueue;

/// A wait queue that can hold wakers to be woken up later.
#[derive(Default, Debug, Clone)]
pub struct Queue {
    waiting: Arc<SegQueue<Waker>>,
}

impl Queue {
    /// Creates a new empty wait queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            waiting: Arc::new(SegQueue::new()),
        }
    }

    /// Wake one waiting waker, if any.
    pub fn wake_one(&self) {
        if let Some(waker) = self.waiting.pop() {
            waker.wake();
        }
    }

    /// Wake all waiting wakers, emptying the queue.
    pub fn wake_all(&self) {
        while let Some(waker) = self.waiting.pop() {
            waker.wake();
        }
    }
}

/// A future that waits on a wait queue until woken up.
#[derive(Debug)]
pub struct WaitFuture<'a> {
    queue: &'a Queue,
    pooled: bool,
}

impl<'a> WaitFuture<'a> {
    /// Creates a new wait future associated with the given wait queue.
    #[must_use]
    pub const fn new(queue: &'a Queue) -> Self {
        Self {
            queue,
            pooled: false,
        }
    }
}

impl Future for WaitFuture<'_> {
    type Output = ();

    /// Polls the wait future, registering the current waker to be woken
    /// up later. If it was already registered, it returns `Poll::Ready(())`,
    /// waking up the task immediately, even if not explicitly woken up by the
    /// queue. Ensuring that the future was woken up by the queue is the caller's
    /// responsibility, and would be too performance-costly to enforce here.
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.pooled {
            return Poll::Ready(());
        }
        self.queue.waiting.push(context.waker().clone());
        self.get_mut().pooled = true;
        Poll::Pending
    }
}

/// Waits on the given wait queue until woken up.
///
/// # Spurious wake-ups
/// This function may return even if not explicitly woken up by the queue. The
/// caller must handle spurious wake-ups by checking the condition again, and
/// calling this function again if necessary.
pub async fn wait(queue: &Queue) {
    WaitFuture::new(queue).await;
}
