use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::Future;

pub mod executor;
pub mod task;
pub mod user;
pub mod waker;

/// A future that yields once before completing. This future can be useful
/// when a proper wake-up mechanism cannot be implemented for X or Y reason,
/// or simply for testing purposes.
#[derive(Debug)]
pub struct YieldFuture {
    polled: bool,
}

impl YieldFuture {
    /// Creates a new `YieldFuture`.
    #[must_use]
    pub fn new() -> Self {
        Self { polled: false }
    }
}

impl Default for YieldFuture {
    fn default() -> Self {
        Self::new()
    }
}

impl Future for YieldFuture {
    type Output = ();

    /// Polls the future. If the future has already been polled, it will
    /// return `Poll::Ready(())`. Otherwise, it will return `Poll::Pending`,
    /// but wake up the waker before doing so. The next time the future is
    /// polled, it will return `Poll::Ready(())`.
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.polled {
            Poll::Ready(())
        } else {
            self.get_mut().polled = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// Yields once before completing.
pub async fn yield_once() {
    YieldFuture::new().await;
}
