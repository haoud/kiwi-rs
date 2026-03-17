use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{BitAnd, BitOr, Not},
};

use crate::arch::x86_64;

/// A marker trait for describing the access type of a port. This is used to
/// indicate whether a port is read-only, write-only, or read-write.
pub trait Access {}

/// A marker trait for read access to a port.
pub trait ReadAccess: Access {}

/// A marker trait for write access to a port.
pub trait WriteAccess: Access {}

/// A marker type for read-only access to a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Read;

impl Access for Read {}
impl ReadAccess for Read {}

/// A marker type for write-only access to a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Write;

impl Access for Write {}
impl WriteAccess for Write {}

/// A marker type for read-write access to a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadWrite;

impl Access for ReadWrite {}
impl ReadAccess for ReadWrite {}
impl WriteAccess for ReadWrite {}

/// A trait for types that can be read from and written to ports.
pub trait IO:
    Sized + Copy + PartialEq + Eq + BitAnd<Output = Self> + BitOr<Output = Self> + Not<Output = Self>
{
    const ZERO: Self;

    /// Write a value to a port.
    ///
    /// # Safety
    /// This function is unsafe because writing to a port can have side effects
    /// that can cause the hardware to do something unexpected, including
    /// violating memory safety. The caller must ensure that writing to the
    /// specified port with the specified value is safe and does not cause any
    /// unintended consequences.
    unsafe fn write(port: u16, value: Self);

    /// Read a value from a port.
    ///
    /// # Safety
    /// This function is unsafe because reading from a port can have side
    /// effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that
    /// reading from the port is safe and will not cause any undefined
    /// behavior.
    unsafe fn read(port: u16) -> Self;
}

impl IO for u8 {
    const ZERO: Self = 0;

    unsafe fn write(port: u16, value: u8) {
        x86_64::instr::outb(port, value);
    }

    unsafe fn read(port: u16) -> u8 {
        x86_64::instr::inb(port)
    }
}

impl IO for u16 {
    const ZERO: Self = 0;

    unsafe fn write(port: u16, value: u16) {
        x86_64::instr::outw(port, value);
    }

    unsafe fn read(port: u16) -> u16 {
        x86_64::instr::inw(port)
    }
}

impl IO for u32 {
    const ZERO: Self = 0;

    unsafe fn write(port: u16, value: u32) {
        x86_64::instr::outd(port, value);
    }

    unsafe fn read(port: u16) -> u32 {
        x86_64::instr::ind(port)
    }
}

/// An enum for describing the timeout behavior when polling a port. This is
/// used to indicate whether to poll indefinitely until the condition is met
/// or to poll for a specified number of iterations before timing out.
///
/// [`Timeout::Infinite`] is not implemented as an actual infinite loop, but
/// rather as a loop that polls for `u32::MAX` iterations, which should be
/// sufficient since polling for that many iterations should take a very long
/// time, especially because I/O ports are slow to access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Timeout {
    Iter(u32),
    Infinite,
}

impl Default for Timeout {
    fn default() -> Self {
        Timeout::Iter(1000)
    }
}

impl From<Timeout> for u32 {
    fn from(timeout: Timeout) -> Self {
        match timeout {
            Timeout::Infinite => u32::MAX,
            Timeout::Iter(iter) => iter,
        }
    }
}

/// An enum for describing whether to poll for bits being set or clear when
/// polling a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Poll {
    Clear,
    Set,
}

/// An enum for describing the result of polling a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PollResult {
    Timeout,
    Success,
    Failure,
}

/// Represents a port that can be read from and/or written to, depending on the
/// access type `A`. This is a wrapper around a port number and a type that
/// implements the `IO` trait (currently `u8`, `u16`, or `u32`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port<T, A> {
    phantom: PhantomData<(T, A)>,
    port: u16,
}

impl<T: IO, A: Access> Port<T, A> {
    /// Create a new port.
    ///
    /// This function is safe because it does not access any hardware but
    /// rather simply encapsulates a port number, a type that implements
    /// the `IO` trait and an access type.
    #[must_use]
    pub const fn new(port: u16) -> Port<T, A> {
        Port {
            port,
            phantom: PhantomData,
        }
    }
}

impl<T: IO, A: ReadAccess> Port<T, A> {
    /// Read a value from the port and then pause for a short time. This is
    /// useful for reading from ports that require a short delay after reading
    /// in order to let enough time pass for the hardware to process the read.
    ///
    /// # Safety
    /// This function is unsafe because reading from a port can have side
    /// effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that reading
    /// from the port is safe and will not cause any undefined behavior.
    #[must_use]
    pub unsafe fn read_and_pause(&self) -> T {
        let data = T::read(self.port);
        pause();
        data
    }

    /// Read a value from the port.
    ///
    /// # Safety
    /// This function is unsafe because reading from a port can have side
    /// effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that reading
    /// from the port is safe and will not cause any undefined behavior.
    #[must_use]
    pub unsafe fn read(&self) -> T {
        T::read(self.port)
    }

    /// Poll a port once by reading its value and checking if all the bits
    /// specified by the mask are set or clear, depending on the value of
    /// `poll`.
    ///
    /// Returns `PollResult::Success` if the condition is met, otherwise
    /// returns `PollResult::Failure`.
    ///
    /// # Safety
    /// This function is unsafe because reading from a port can have side
    /// effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that
    /// reading from the port is safe and does not cause any undefined
    /// behavior.
    pub unsafe fn poll_once(&self, bits: T, poll: Poll) -> PollResult {
        let data = T::read(self.port);
        let success = match poll {
            Poll::Clear => (data & bits) == T::ZERO,
            Poll::Set => (data & bits) == bits,
        };

        if success {
            PollResult::Success
        } else {
            PollResult::Failure
        }
    }

    /// Poll a port by repeatedly reading its value and checking if all the
    /// bits specified by the mask are set or clear, depending on the value of
    /// `poll`, until the condition is met. If a timeout is specified, the
    /// function will return after the specified number of iterations even if the
    /// condition is not met.
    ///
    /// If `timeout` is `None`, the function will poll indefinitely until the
    /// condition is met.
    ///
    /// Returns `PollResult::Success` if the condition is met, otherwise
    /// returns `PollResult::Timeout` if the timeout is reached.
    ///
    /// # Safety
    /// This function is unsafe because reading from a port can have side
    /// effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that
    /// reading from the port is safe and will not cause any undefined
    /// behavior.
    pub unsafe fn poll_until(&self, bits: T, poll: Poll, timeout: Timeout) -> PollResult {
        for _ in 0..u32::from(timeout) {
            match self.poll_once(bits, poll) {
                PollResult::Success => return PollResult::Success,
                PollResult::Timeout => unreachable!(),
                PollResult::Failure => (),
            }
        }

        PollResult::Timeout
    }
}

impl<T: IO, A: WriteAccess> Port<T, A> {
    /// Write a value to the port, then pause for a short time. This is useful
    /// for writing to ports that require a short delay after writing in order
    /// to let enough time pass for the hardware to process the write.
    ///
    /// # Safety
    /// This function is unsafe because writing to a port can have side effects
    /// that can cause the hardware to do something unexpected, including
    /// violating memory safety. The caller must ensure that writing to the
    /// specified port with the specified value is safe and does not cause any
    /// unintended consequences.
    pub unsafe fn write_and_pause(&self, value: T) {
        T::write(self.port, value);
        pause();
    }

    /// Write a value to the port.
    ///
    /// # Safety
    /// This function is unsafe because writing to a port can have side effects
    /// that can cause the hardware to do something unexpected, including
    /// violating memory safety. The caller must ensure that writing to the
    /// specified port with the specified value is safe and does not cause any
    /// unintended consequences.
    pub unsafe fn write(&self, value: T) {
        T::write(self.port, value);
    }
}

impl<T: IO, A: ReadAccess + WriteAccess> Port<T, A> {
    /// Clear the specified bits in the port by reading the current value and
    /// writing back the value with the specified bits cleared.
    ///
    /// # Safety
    /// This function is unsafe because reading from and writing to a port can
    /// have side effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that reading
    /// from and writing to the specified port with the specified value is safe
    /// and does not cause any unintended consequences.
    pub unsafe fn clear_bits(&self, bits: T) {
        let data = T::read(self.port);
        T::write(self.port, data & !bits);
    }

    /// Set the specified bits in the port by reading the current value and
    /// writing back the value with the specified bits set.
    ///
    /// # Safety
    /// This function is unsafe because reading from and writing to a port can
    /// have side effects that can cause the hardware to do something unexpected,
    /// including violating memory safety. The caller must ensure that reading
    /// from and writing to the specified port with the specified value is safe
    /// and does not cause any unintended consequences.
    pub unsafe fn set_bits(&self, bits: T) {
        let data = T::read(self.port);
        T::write(self.port, data | bits);
    }
}

/// Pause for a short time. This is useful for writing to ports that require a
/// short delay after writing in order to let enough time pass for the hardware
/// to process the write.
///
/// # Safety
/// Currently this function is implemented by writing to port 0x80, which is
/// (was ?) used by Linux, but it may be fragile as it assumes that the port
/// 0x80 is not used by the hardware. This is why this function is marked as
/// unsafe, though it should be safe in practice.
pub unsafe fn pause() {
    x86_64::instr::outb(0x80, 0);
}
