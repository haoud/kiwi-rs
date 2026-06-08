use crate::time::duration::Duration;

pub mod duration;
pub mod instant;
pub mod jiffies;
pub mod timer;

/// A timer frequency, in Hz.
///
/// This newtype is used to represent the frequency of a timer with a sanity
/// check on the value, to ensure that it is within the acceptable range for
/// the hardware timer and that it is not too low or too high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerFrequency(u32);

impl TimerFrequency {
    pub const MAX: Self = Self(10_000);
    pub const MIN: Self = Self(1);

    /// Creates a new `TimerFrequency` with the given frequency in Hz.
    ///
    /// # Panics
    /// Panics if the given frequency is less than 1 Hz or greater than
    /// 10,000 Hz, which should be a reasonable range for most hardware
    /// timers and is flexible enough for all use cases in the kernel.
    #[must_use]
    pub const fn new(frequency: u32) -> Self {
        if frequency < Self::MIN.0 {
            panic!("Timer frequency cannot be zero");
        } else if frequency > Self::MAX.0 {
            panic!("Timer frequency is too high");
        } else {
            Self(frequency)
        }
    }

    /// Creates a new `TimerFrequency` from the given duration, by computing the
    /// frequency as the inverse of the duration.
    ///
    /// # Panics
    /// Panics if the given duration is less than 100 microseconds or greater than
    /// 1 second, which should be a reasonable range for most hardware timers and
    /// is flexible enough for all use cases in the kernel.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_duration(duration: Duration) -> Self {
        if duration.as_nanos() < Self::MIN.as_duration().as_nanos() {
            panic!("Timer tick duration is too short");
        } else if duration.as_nanos() > Self::MAX.as_duration().as_nanos() {
            panic!("Timer tick duration is too long");
        } else {
            Self::new((1_000_000_000 / duration.as_nanos()) as u32)
        }
    }

    /// Converts this `TimerFrequency` to a `Duration` to represent the
    /// duration of a timer tick.
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / self.0 as u64)
    }

    /// Returns the frequency in Hz of this `TimerFrequency`.
    #[must_use]
    pub const fn as_hertz(&self) -> u32 {
        self.0
    }
}
