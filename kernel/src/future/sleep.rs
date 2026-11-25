use core::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::time::{
    duration::Duration,
    instant::Instant,
    timer::{self, TimerGuard},
};

/// A future that completes once a deadline has been reached. It registers a
/// one-shot timer that wakes up the task when the deadline expires, and the
/// timer is cancelled if the future is dropped before completing.
pub struct Sleep {
    /// The instant at which the future will complete.
    deadline: Instant,

    /// The guard of the timer that will wake up the task, if a timer has
    /// already been registered.
    timer: Option<TimerGuard>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() >= self.deadline {
            return Poll::Ready(());
        }

        // Only register the timer during the first poll. The waker of a task
        // never changes in our executor, so there is no need to register a new
        // timer if the task was spuriously woken up.
        if self.timer.is_none() {
            let waker = cx.waker().clone();
            let deadline = self.deadline;
            self.timer = Some(timer::schedule_at(deadline, move |_| {
                waker.wake_by_ref();
            }));
        }
        Poll::Pending
    }
}

/// Returns a future that completes after the given duration has elapsed.
#[must_use]
pub fn sleep(duration: Duration) -> Sleep {
    Sleep {
        deadline: Instant::now() + duration,
        timer: None,
    }
}
