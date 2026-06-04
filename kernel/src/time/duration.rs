use core::{
    iter::Sum,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

/// A duration of time, represented as a number of nanoseconds. This is a
/// simple wrapper that is more convenient to work with than a raw number of
/// nanoseconds, and is more efficient than the `core::time::Duration` type
/// since it only needs to store a single u64 value.
///
/// Storing a nanosecond duration as a single u64 value allows us to represent
/// durations up to approximately 584 years, which should enough for all use
/// cases in the kernel, while still being efficient to store and manipulate.
///
/// # Overflow behavior
/// The `Duration` type does not perform any overflow checks on its operations,
/// so it is the caller's responsibility to ensure that the durations being
/// added or subtracted do not overflow the maximum representable duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    nanos: u64,
}

impl Duration {
    /// Returns a `Duration` that represents zero time.
    #[must_use]
    pub const fn none() -> Self {
        Self { nanos: 0 }
    }

    /// Creates a new `Duration` from the given number of nanoseconds.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Creates a new `Duration` from the given number of microseconds.
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros * 1_000,
        }
    }

    /// Creates a new `Duration` from the given number of milliseconds.
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            nanos: millis * 1_000_000,
        }
    }

    /// Creates a new `Duration` from the given number of seconds.
    #[must_use]
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nanos: secs * 1_000_000_000,
        }
    }

    /// Returns the total number of nanoseconds represented by this `Duration`.
    #[must_use]
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Returns the total number of microseconds represented by this
    /// `Duration`, rounding down to the nearest microsecond.
    #[must_use]
    pub const fn as_micros(&self) -> u64 {
        self.nanos / 1_000
    }

    /// Returns the total number of milliseconds represented by this
    /// `Duration`, rounding down to the nearest millisecond.
    #[must_use]
    pub const fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Returns the total number of seconds represented by this `Duration`,
    /// rounding down to the nearest second.
    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            nanos: self.nanos + rhs.nanos,
        }
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            nanos: self.nanos - rhs.nanos,
        }
    }
}

impl Mul<u32> for Duration {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        Self {
            nanos: self.nanos * u64::from(rhs),
        }
    }
}

impl Div<u32> for Duration {
    type Output = Self;

    fn div(self, rhs: u32) -> Self::Output {
        Self {
            nanos: self.nanos / u64::from(rhs),
        }
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.nanos += rhs.nanos;
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        self.nanos -= rhs.nanos;
    }
}

impl MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        self.nanos *= u64::from(rhs);
    }
}

impl DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        self.nanos /= u64::from(rhs);
    }
}

impl Sum for Duration {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::from_nanos(0), |acc, x| acc + x)
    }
}
