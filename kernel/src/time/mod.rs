use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

use crate::arch;

/// A measurement of a monotonically nondecreasing clock. This is very
/// similar to `std::time::Instant`, but tailored for kernel use.
///
/// # Panics
/// Operations on `Instant` are guaranteed to never panic in real-world usage,
/// but MAY panic in extremely unlikely edge cases, such as the underlying
/// architecture's timer overflowing an u64 nanosecond representation, which
/// would require the system to be running for approximately 584 years without
/// rebooting. I will be dead way before that happens, so I will let future
/// maintainers deal with that problem if it ever arises :)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    /// Returns an instant corresponding to "now".
    ///
    /// # Panics
    /// This function MAY panic if the underlying architecture's timer overflows
    /// an u64 nanosecond representation. This is highly unlikely to happen in
    /// practice, since it would require the system to be running for hundreds
    /// of years without rebooting.
    #[must_use]
    pub fn now() -> Self {
        Instant(arch::timer::current_time_ticks() * arch::timer::internal_tick())
    }

    /// Returns the duration elapsed since this instant.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }

    /// Returns whether this instant has already passed.
    #[must_use]
    pub fn has_passed(&self) -> bool {
        Instant::now() >= *self
    }

    /// Returns the duration elapsed since the earlier instant. If `earlier`
    /// is later than `self`, the returned duration will be zero.
    #[must_use]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }

    /// Returns the duration until the later instant. If `later` is earlier
    /// than `self`, the returned duration will be zero.
    #[must_use]
    pub fn duration_until(&self, later: Instant) -> Duration {
        Duration::from_nanos(later.0.saturating_sub(self.0))
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    #[allow(clippy::cast_possible_truncation)]
    fn add(self, duration: Duration) -> Instant {
        Instant(self.0.saturating_add(duration.as_nanos() as u64))
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    #[allow(clippy::cast_possible_truncation)]
    fn sub(self, duration: Duration) -> Instant {
        Instant(self.0.saturating_sub(duration.as_nanos() as u64))
    }
}

impl AddAssign<Duration> for Instant {
    #[allow(clippy::cast_possible_truncation)]
    fn add_assign(&mut self, duration: Duration) {
        self.0 = self.0.saturating_add(duration.as_nanos() as u64);
    }
}

impl SubAssign<Duration> for Instant {
    #[allow(clippy::cast_possible_truncation)]
    fn sub_assign(&mut self, duration: Duration) {
        self.0 = self.0.saturating_sub(duration.as_nanos() as u64);
    }
}

/// Measures the time taken to execute the provided closure, returning both
/// the result of the closure and the duration it took to execute.
#[must_use]
pub fn spent_into<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let result = f();
    (result, start.elapsed())
}
