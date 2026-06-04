use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::{config, library::lock::spin::Spinlock, time::duration::Duration};

/// A number of a timer ticks. Ticks are the basic unit of time measurement in
/// the kernel, and their duration is fixed by the timer frequency configured
/// in `config::TIMER_HZ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(u64);

impl Tick {
    /// The duration of a single tick, which is determined by the timer
    /// frequency configured in `config::TIMER_HZ`.
    pub const DURATION: Duration = config::TIMER_HZ.as_duration();

    /// Returns a `Tick` with a value of zero, representing the starting
    /// point for measuring time in ticks.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Increment this `Tick` by one tick.
    pub const fn increment(&mut self) {
        self.0 += 1;
    }

    /// Get the tick value as a `u64`.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<Duration> for Tick {
    /// Converts a `Duration` to a `Tick` by dividing the duration by the
    /// duration of a single tick, rounding up to the nearest tick if the
    /// duration is not an exact multiple of the tick duration.
    fn from(duration: Duration) -> Self {
        Self(duration.as_nanos().div_ceil(Tick::DURATION.as_nanos()))
    }
}

impl From<Tick> for Duration {
    /// Converts a `Tick` to a `Duration` by multiplying the tick value by the
    /// duration of a single tick.
    fn from(tick: Tick) -> Self {
        Self::from_nanos(tick.as_u64() * Tick::DURATION.as_nanos())
    }
}

impl From<Tick> for u64 {
    /// Converts a `Tick` to a `u64` by returning the tick value as a `u64`.
    fn from(tick: Tick) -> Self {
        tick.as_u64()
    }
}

impl Add<u64> for Tick {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<u64> for Tick {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl AddAssign<u64> for Tick {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl SubAssign<u64> for Tick {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

/// The number of jiffies since the system booted. This is incremented by the
/// timer interrupt handler at each timer tick, and can be used to measure the
/// time elapsed since the system booted or since a specific event. This is a
/// global variable protected by a spinlock to ensure safe concurrent access,
/// and only the BSP should increment it at each timer tick.
static JIFFIES: Spinlock<Tick> = Spinlock::new(Tick::zero());

/// Increment the number of jiffies by one tick. This should be called by the
/// timer interrupt handler at each timer tick to keep track of the time
/// elapsed since the system booted. Only once core should call this function
/// per timer tick.
pub fn increment_jiffies() {
    JIFFIES.lock_irq_safe().increment();
}

/// Returns the number of jiffies since the system booted.
#[must_use]
pub fn jiffies() -> Tick {
    *JIFFIES.lock_irq_safe()
}
