//! A driver for a PS/2 keyboard connected to the first port of the i8042
//! controller. The controller is configured to translate the scancodes sent by
//! the keyboard into the scancode set 1, which is decoded in the interrupt
//! handler with `pc-keyboard` and pushed into the keyboard event queue.
use bitflags::bitflags;
use pc_keyboard::{ScancodeSet, ScancodeSet1};

use crate::{
    arch::x86_64::{
        apic,
        io::{InlinePort, Poll, PollResult, ReadWrite, Timeout},
    },
    driver::keyboard,
    library::lock::spin::Spinlock,
};

/// The interrupt vector of the keyboard: the ISA IRQ 1, routed through the
/// I/O APIC.
pub const IRQ_VECTOR: u8 = apic::io::IRQ_BASE + 1;

/// The data port of the controller, used to exchange bytes with the device
/// and to read or write the configuration byte.
static DATA: InlinePort<0x60, u8, ReadWrite> = InlinePort::new();

/// The status register (when read) and the command register (when written)
/// of the controller.
static STATUS_COMMAND: InlinePort<0x64, u8, ReadWrite> = InlinePort::new();

bitflags! {
    /// The status register of the controller.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Status: u8 {
        /// The output buffer is full: a byte can be read from the data port.
        const OUTPUT_FULL = 1 << 0;

        /// The input buffer is full: the controller has not processed the
        /// last byte written yet.
        const INPUT_FULL = 1 << 1;
    }
}

bitflags! {
    /// The configuration byte of the controller.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Config: u8 {
        /// Enables the interrupt of the first port.
        const PORT1_IRQ = 1 << 0;

        /// Enables the interrupt of the second port.
        const PORT2_IRQ = 1 << 1;

        /// Set by the firmware once the system has passed its self test.
        const SYSTEM_FLAG = 1 << 2;

        /// Disables the clock of the first port. It is set by
        /// [`Command::DisablePort1`] and cleared by [`Command::EnablePort1`].
        const PORT1_CLOCK_DISABLED = 1 << 4;

        /// Disables the clock of the second port.
        const PORT2_CLOCK_DISABLED = 1 << 5;

        /// Enables the translation of the scancodes sent by the keyboard into
        /// the scancode set 1.
        const TRANSLATION = 1 << 6;
    }
}

/// A command sent to the controller through its command register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Command {
    /// Reads the configuration byte, which is then available on the data
    /// port.
    ReadConfig = 0x20,

    /// Writes the next byte written to the data port into the configuration
    /// byte.
    WriteConfig = 0x60,

    /// Disables the second port (usually the mouse).
    DisablePort2 = 0xA7,

    /// Runs the self test of the controller, which answers on the data port.
    TestController = 0xAA,

    /// Tests the first port, the result is available on the data port.
    TestPort1 = 0xAB,

    /// Disables the first port (the keyboard).
    DisablePort1 = 0xAD,

    /// Enables the first port (the keyboard).
    EnablePort1 = 0xAE,

    /// Pulses the reset line of the CPU, which restarts the computer.
    PulseReset = 0xFE,
}

/// A command sent to the keyboard through the data port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DeviceCommand {
    /// Allows the keyboard to send the scancodes of the keys pressed.
    EnableScanning = 0xF4,

    /// Resets the keyboard, which answers with an acknowledgement then with
    /// the result of its self test.
    Reset = 0xFF,
}

/// A byte sent on the data port by the controller or the keyboard in response
/// to a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Response(u8);

impl Response {
    /// The test of a port succeeded.
    pub const PORT_TEST_PASSED: Response = Response(0x00);

    /// The self test of the controller succeeded.
    pub const CONTROLLER_TEST_PASSED: Response = Response(0x55);

    /// The self test of the keyboard succeeded.
    pub const SELF_TEST_PASSED: Response = Response(0xAA);

    /// The keyboard acknowledged a command.
    pub const ACK: Response = Response(0xFA);
}

/// The number of times the status register is polled before giving up. Since
/// reading an I/O port takes about one microsecond, this is roughly 100 ms.
const TIMEOUT: Timeout = Timeout::Iter(100_000);

/// The reasons why the keyboard cannot be initialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// The controller did not answer in time: there is probably no PS/2
    /// controller in the system.
    Timeout,

    /// The self test of the controller failed.
    ControllerTestFailed(Response),

    /// The test of the first port failed: there is probably no keyboard
    /// connected to it.
    PortTestFailed(Response),

    /// The keyboard did not acknowledge a command.
    NoAck(DeviceCommand),
}

/// The decoder of the scancodes received from the keyboard.
static DECODER: Spinlock<ScancodeSet1> = Spinlock::new(ScancodeSet1::new());

/// Initializes the controller and the keyboard, and enables the keyboard
/// interrupt.
///
/// # Errors
/// Returns an error if the controller or the keyboard does not respond as
/// expected. In this case, the keyboard interrupt is not enabled.
///
/// # Safety
/// This function must be called only once, after the I/O APIC has been set
/// up, and the keyboard interrupt must be handled by calling [`handle_irq`].
pub unsafe fn setup() -> Result<(), Error> {
    // Disable the devices while the controller is configured, so that they
    // do not send any data in the meantime, and discard any pending byte.
    send_command(Command::DisablePort1)?;
    send_command(Command::DisablePort2)?;
    flush();

    // Disable the interrupts and enable the translation to scancode set 1.
    let config = (read_config()? - (Config::PORT1_IRQ | Config::PORT2_IRQ)) | Config::TRANSLATION;
    write_config(config)?;

    send_command(Command::TestController)?;
    match read_response()? {
        Response::CONTROLLER_TEST_PASSED => {}
        response => return Err(Error::ControllerTestFailed(response)),
    }

    // The self test may reset the controller on some hardware, so write the
    // configuration again.
    write_config(config)?;

    send_command(Command::TestPort1)?;
    match read_response()? {
        Response::PORT_TEST_PASSED => {}
        response => return Err(Error::PortTestFailed(response)),
    }
    send_command(Command::EnablePort1)?;

    // Reset the keyboard to put it in a known state. Some keyboards do not
    // support the reset command, so a failure is not fatal.
    match send_to_device(DeviceCommand::Reset) {
        Ok(()) => match read_response() {
            Ok(Response::SELF_TEST_PASSED) => {}
            result => log::warn!("PS/2: unexpected keyboard reset result: {result:?}"),
        },
        Err(error) => log::warn!("PS/2: keyboard reset failed: {error:?}"),
    }
    send_to_device(DeviceCommand::EnableScanning)?;
    flush();

    // Writing the configuration byte read while the port was disabled would
    // disable the port again: clear the corresponding bit.
    write_config((config - Config::PORT1_CLOCK_DISABLED) | Config::PORT1_IRQ)?;
    apic::io::enable_irq(IRQ_VECTOR);

    // The interrupt is edge-triggered: if a byte arrived before the interrupt
    // was enabled, the interrupt line is already raised and no edge will be
    // seen until this byte is read. Read it to make sure the line is lowered.
    flush();
    log::info!("PS/2: keyboard initialized");
    Ok(())
}

/// Handles the keyboard interrupt: reads the byte sent by the keyboard,
/// decodes it and pushes the resulting key event, if any, into the keyboard
/// event queue. The caller is responsible for acknowledging the interrupt.
pub fn handle_irq() {
    if !read_status().contains(Status::OUTPUT_FULL) {
        // Spurious interrupt: there is nothing to read.
        return;
    }

    // SAFETY: The output buffer is full, so reading the data port only
    // retrieves the byte sent by the keyboard.
    let byte = unsafe { DATA.read() };
    let event = DECODER.lock_irq_safe().advance_state(byte);
    match event {
        Ok(Some(event)) => keyboard::push_event(event),
        // The byte is a prefix of a scancode made of several bytes (e.g. the
        // `0xE0` prefix of the extended keys): wait for the next bytes.
        Ok(None) => {}
        // Acknowledgements and unknown keys are not key events.
        Err(error) => log::debug!("PS/2: ignoring byte {byte:#04x} ({error:?})"),
    }
}

/// Resets the computer by pulsing the reset line of the CPU, which is
/// connected to the controller on PC compatible hardware. Returns only if the
/// controller did not reset the computer.
pub fn pulse_reset_line() {
    _ = send_command(Command::PulseReset);
}

/// Reads the status register of the controller.
fn read_status() -> Status {
    // SAFETY: Reading the status register has no side effect.
    Status::from_bits_retain(unsafe { STATUS_COMMAND.read() })
}

/// Reads the configuration byte of the controller.
fn read_config() -> Result<Config, Error> {
    send_command(Command::ReadConfig)?;
    Ok(Config::from_bits_retain(read_data()?))
}

/// Writes the configuration byte of the controller.
fn write_config(config: Config) -> Result<(), Error> {
    send_command(Command::WriteConfig)?;
    write_data(config.bits())
}

/// Sends a command to the keyboard and waits for its acknowledgement.
fn send_to_device(command: DeviceCommand) -> Result<(), Error> {
    write_data(command as u8)?;
    match read_response()? {
        Response::ACK => Ok(()),
        _ => Err(Error::NoAck(command)),
    }
}

/// Sends a command to the controller.
fn send_command(command: Command) -> Result<(), Error> {
    wait_input_empty()?;
    // SAFETY: The commands sent by this driver only configure the controller,
    // except `PulseReset` whose effect (restarting the computer) is expected.
    unsafe { STATUS_COMMAND.write(command as u8) };
    Ok(())
}

/// Writes a byte to the data port, once the controller is ready to accept it.
fn write_data(byte: u8) -> Result<(), Error> {
    wait_input_empty()?;
    // SAFETY: The controller is ready to receive a byte.
    unsafe { DATA.write(byte) };
    Ok(())
}

/// Reads the response to a command from the data port.
fn read_response() -> Result<Response, Error> {
    read_data().map(Response)
}

/// Reads a byte from the data port, once one is available.
fn read_data() -> Result<u8, Error> {
    // SAFETY: Polling the status register has no side effect, and the data
    // port is read only once a byte is available.
    unsafe {
        match STATUS_COMMAND.poll_until(Status::OUTPUT_FULL.bits(), Poll::Set, TIMEOUT) {
            PollResult::Success => Ok(DATA.read()),
            _ => Err(Error::Timeout),
        }
    }
}

/// Waits until the controller has processed the last byte written to it.
fn wait_input_empty() -> Result<(), Error> {
    // SAFETY: Polling the status register has no side effect.
    match unsafe { STATUS_COMMAND.poll_until(Status::INPUT_FULL.bits(), Poll::Clear, TIMEOUT) } {
        PollResult::Success => Ok(()),
        _ => Err(Error::Timeout),
    }
}

/// Discards the bytes waiting in the output buffer of the controller.
fn flush() {
    for _ in 0..16 {
        if !read_status().contains(Status::OUTPUT_FULL) {
            break;
        }
        // SAFETY: A byte is available, so reading the data port only
        // retrieves it.
        _ = unsafe { DATA.read() };
    }
}
