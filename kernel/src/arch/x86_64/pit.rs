use crate::arch::x86_64::io::{Port, ReadWrite, Write};

/// The internal frequency of the PIT, in Hz. This is the frequency of the
/// internal oscillator that drives the PIT, and is used to calculate the
/// number of ticks per second for a given divisor.
pub const INTERNAL_FREQ: usize = 1_193_180;

/// The number of nanoseconds between each PIT internal tick.
pub const PIT_TICK_NS: usize = 1_000_000_000 / INTERNAL_FREQ;

/// Channel available for the PIT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Channel {
    Ch0 = 0,
    Ch1 = 1,
    Ch2 = 2,
}

/// Access mode for the PIT command port.
///
/// The access mode determines how the PIT will read or write the counter value
/// for the specified channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Access {
    /// Access the counter latch.
    Latch = 0b00 << 4,

    /// Access the low byte only.
    Lobyte = 0b01 << 4,

    /// Access the high byte only.
    Hibyte = 0b10 << 4,

    /// Access the low byte first, then the high byte after.
    LoHibyte = 0b11 << 4,
}

/// Operating mode for the PIT command port. Only the one-shot mode is used
/// in this implementation, and other modes are not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OperatingMode {
    None = 0b000 << 1,
    OneShot = 0b001 << 1,
}

static CHANNEL0: Port<u8, ReadWrite> = Port::new(0x40);
static CHANNEL1: Port<u8, ReadWrite> = Port::new(0x41);
static CHANNEL2: Port<u8, ReadWrite> = Port::new(0x42);
static CMD: Port<u8, Write> = Port::new(0x43);
static SCP_B: Port<u8, ReadWrite> = Port::new(0x61);

/// Configure the channel 2 of the PIT to generate a one-shot timer that will
/// trigger after `ms` milliseconds since the function is called.
///
/// This function should not be called if the PC speaker is in use, as it will
/// disable the speaker and disable the timer 2 gate, which may interfere with
/// the speaker's functionality.
///
/// # Panics
/// Panics if `ms` is not in the range (0, 100).
#[allow(clippy::cast_possible_truncation)]
pub fn prepare_sleep(ms: usize) {
    assert!(ms < 100, "ms must be less than 100");
    assert!(ms > 0, "ms must be greater than 0");
    let counter = (ms * 1_000_000) / PIT_TICK_NS;

    // Clear the speaker bit (bit 0) and the timer 2 gate bit (bit 1) in the
    // System Control Port B to ensure that the speaker is disabled and the
    // timer 2 is not gated by the speaker.
    // SAFETY: This should not cause any side effects that could lead to
    // undefined behavior or memory unsafety, as it only modifies the bits
    // related to the speaker and timer 2 gate.
    unsafe {
        SCP_B.clear_bits(0b11);
    }

    write_command(Channel::Ch2, Access::LoHibyte, OperatingMode::OneShot);
    write_channel(Channel::Ch2, counter as u16);
}

/// Perform a sleep after the PIT has been configured with [`prepare_sleep`].
///
/// This function will block until the PIT internal counter reaches 0 on the
/// channel 2. If no call to `prepare_sleep` has been made, the behavior of
/// this function is undefined, and may block indefinitely or return
/// immediately depending on the state of the PIT.
pub fn perform_sleep() {
    while read_counter(Channel::Ch2) > 0 {
        core::hint::spin_loop();
    }
}

/// Get the port associated with the specified PIT channel.
#[must_use]
fn get_channel_port(channel: Channel) -> &'static Port<u8, ReadWrite> {
    match channel {
        Channel::Ch0 => &CHANNEL0,
        Channel::Ch1 => &CHANNEL1,
        Channel::Ch2 => &CHANNEL2,
    }
}

/// Read the current value of the counter for the specified PIT channel.
///
/// This function sends a latch command to the PIT to capture the current
/// counter, and then reads the low byte and high byte from the channel's
/// port to construct the full 16-bit counter value. Using a latch command
/// allows to read the counter value properly without worrying about the
/// counter changing between reading the low byte and the high byte.
///
/// This function relies on reading and writing to I/O ports and should be used
/// with caution since I/O port operations are slow on `x86_64`.
#[must_use]
pub fn read_counter(channel: Channel) -> u16 {
    write_command(channel, Access::Latch, OperatingMode::None);

    // SAFETY: Reading from a PIT channel port should be safe and should not
    // cause any side effects that could lead to undefined behavior or memory
    // unsafety.
    unsafe {
        let port = get_channel_port(channel);
        let lo = port.read();
        let hi = port.read();
        (u16::from(hi) << 8) | u16::from(lo)
    }
}

/// Write a command to the PIT command port to configure the specified channel
/// with the given access mode and operating mode.
fn write_command(channel: Channel, access: Access, mode: OperatingMode) {
    let command = ((channel as u8) << 6) | access as u8 | mode as u8;
    // SAFETY: Writing to the command port should be safe and should not
    // cause any side effects that could lead to undefined behavior or
    // memory unsafety.
    unsafe {
        CMD.write_and_pause(command);
    }
}

/// Write a 16-bit value to the specified PIT channel.
///
/// The value is split into two 8-bit parts and written to the channel's port,
/// therefore this function assumes that the access mode for the channel has
/// been set to `LoHibyte`.
fn write_channel(channel: Channel, value: u16) {
    let port = get_channel_port(channel);

    // SAFETY: Writing to a PIT channel port should be safe and should not
    // cause any side effects that could lead to undefined behavior or memory
    // unsafety.
    unsafe {
        port.write_and_pause((value & 0xFF) as u8);
        port.write_and_pause((value >> 8) as u8);
    }
}
