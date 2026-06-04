use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::time::{self, duration::Duration};

/// A measurement of a monotonically nondecreasing clock.
///
/// The `Instant` type represents a specific point in time, and is used to
/// measure durations and intervals. Internally, it is represented as a
/// `Duration` since the kernel's boot time, which is a convenient way to
/// represent a point in time that can be compared and manipulated using the
/// `Duration` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(Duration);

impl Instant {
    /// Returns the current kernel tick count as an `Instant`. This is a simple
    /// wrapper around the `time::jiffies::jiffies()` function that converts
    /// the tick count into an `Instant`.
    ///
    /// The main inconvenience of using the jiffies count as the internal
    /// representation of an `Instant` is that it has a relatively low
    /// resolution depending on the timer frequency, which can lead to
    /// inaccuracies when measuring durations that are shorter than the timer
    /// tick period. However, this allows to have a simple and very efficient
    /// implementation of the `Instant` that should be very fast.
    #[must_use]
    pub fn now() -> Instant {
        Self(Duration::from(time::jiffies::jiffies()))
    }

    /// Returns the duration that has elapsed since this `Instant`. This is a
    /// simple wrapper around the `Instant::now()` function that calculates the
    /// difference between the current time and this `Instant` to get the elapsed
    /// duration.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Instant::now() - *self
    }

    /// Returns the duration between this `Instant` and an earlier `Instant`.
    /// If the earlier `Instant` is actually later than this `Instant`, then
    /// the returned `Duration` saturates at zero instead of underflowing or
    /// panicking.
    #[must_use]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        if *self >= earlier {
            *self - earlier
        } else {
            Duration::none()
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs;
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 -= rhs;
    }
}
