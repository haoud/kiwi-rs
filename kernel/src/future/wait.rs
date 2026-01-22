use alloc::sync::Arc;
use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};
use crossbeam::queue::SegQueue;

/// A wait queue that can hold wakers to be woken up later.
#[derive(Default, Debug, Clone)]
pub struct Queue {
    waiting: Arc<SegQueue<Waker>>,
    poisoned: Arc<AtomicBool>,
}

impl Queue {
    /// Creates a new empty wait queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            waiting: Arc::new(SegQueue::new()),
            poisoned: Arc::new(AtomicBool::new(false)),
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

    /// Poisons the wait queue, causing all future waiters to be woken up
    /// immediately without sleeping. This is useful when the resource being
    /// waited on is no longer available.
    pub fn poison(&self) {
        self.poisoned.store(true, Ordering::SeqCst);
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
    ///
    /// # Wait queue poisoning
    /// If the wait queue was poisoned, the future returns `Poll::Ready(())`
    /// immediately, without waiting. This allows tasks to be  woken up when
    /// the resource they were waiting on is no longer available, without
    /// getting stuck forever, but may lead to spurious wake-ups if the
    /// queue was poisoned after registering the waker.
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.pooled {
            return Poll::Ready(());
        }
        self.queue.waiting.push(context.waker().clone());

        // Check if the queue was poisoned after registering the waker to
        // ensure that we will not miss a wake-up and prevent getting stuck
        // forever. However, this may lead to spurious wake-ups if the queue
        // was poisoned between the check and the registration, but should not
        // be a problem since this situation should be very rare.
        if self.queue.poisoned.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }
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
