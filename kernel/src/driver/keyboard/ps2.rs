//! A driver for a PS/2 keyboard connected to the first port of the i8042
//! controller. The controller is configured to translate the scancodes sent by
//! the keyboard into the scancode set 1, which is decoded in the interrupt
//! handler with `pc-keyboard` and pushed into the keyboard event queue.
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

/// The output buffer is full: a byte can be read from the data port.
const STATUS_OUTPUT_FULL: u8 = 1 << 0;

/// The input buffer is full: the controller has not processed the last byte
/// written yet.
const STATUS_INPUT_FULL: u8 = 1 << 1;

const COMMAND_READ_CONFIG: u8 = 0x20;
const COMMAND_WRITE_CONFIG: u8 = 0x60;
const COMMAND_DISABLE_PORT2: u8 = 0xA7;
const COMMAND_TEST_CONTROLLER: u8 = 0xAA;
const COMMAND_TEST_PORT1: u8 = 0xAB;
const COMMAND_DISABLE_PORT1: u8 = 0xAD;
const COMMAND_ENABLE_PORT1: u8 = 0xAE;
const COMMAND_PULSE_RESET: u8 = 0xFE;

const CONFIG_PORT1_IRQ: u8 = 1 << 0;
const CONFIG_PORT2_IRQ: u8 = 1 << 1;
const CONFIG_PORT1_CLOCK_DISABLED: u8 = 1 << 4;
const CONFIG_TRANSLATION: u8 = 1 << 6;

const CONTROLLER_TEST_PASSED: u8 = 0x55;
const PORT_TEST_PASSED: u8 = 0x00;

const DEVICE_RESET: u8 = 0xFF;
const DEVICE_ENABLE_SCANNING: u8 = 0xF4;
const DEVICE_ACK: u8 = 0xFA;
const DEVICE_SELF_TEST_PASSED: u8 = 0xAA;

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
    ControllerTestFailed(u8),

    /// The test of the first port failed: there is probably no keyboard
    /// connected to it.
    PortTestFailed(u8),

    /// The keyboard did not acknowledge a command.
    NoAck(u8),
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
    send_command(COMMAND_DISABLE_PORT1)?;
    send_command(COMMAND_DISABLE_PORT2)?;
    flush();

    // Disable the interrupts and enable the translation to scancode set 1.
    send_command(COMMAND_READ_CONFIG)?;
    let config = (read_data()? & !(CONFIG_PORT1_IRQ | CONFIG_PORT2_IRQ)) | CONFIG_TRANSLATION;
    write_config(config)?;

    send_command(COMMAND_TEST_CONTROLLER)?;
    match read_data()? {
        CONTROLLER_TEST_PASSED => {}
        result => return Err(Error::ControllerTestFailed(result)),
    }

    // The self test may reset the controller on some hardware, so write the
    // configuration again.
    write_config(config)?;

    send_command(COMMAND_TEST_PORT1)?;
    match read_data()? {
        PORT_TEST_PASSED => {}
        result => return Err(Error::PortTestFailed(result)),
    }
    send_command(COMMAND_ENABLE_PORT1)?;

    // Reset the keyboard to put it in a known state. Some keyboards do not
    // support the reset command, so a failure is not fatal.
    match send_to_device(DEVICE_RESET) {
        Ok(()) => match read_data() {
            Ok(DEVICE_SELF_TEST_PASSED) => {}
            result => log::warn!("PS/2: unexpected keyboard reset result: {result:?}"),
        },
        Err(error) => log::warn!("PS/2: keyboard reset failed: {error:?}"),
    }
    send_to_device(DEVICE_ENABLE_SCANNING)?;
    flush();

    // Writing the configuration byte read while the port was disabled would
    // disable the port again: clear the corresponding bit.
    write_config((config & !CONFIG_PORT1_CLOCK_DISABLED) | CONFIG_PORT1_IRQ)?;
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
    // SAFETY: Reading the status register has no side effect.
    let status = unsafe { STATUS_COMMAND.read() };
    if status & STATUS_OUTPUT_FULL == 0 {
        return;
    }

    // SAFETY: The output buffer is full, so reading the data port only
    // retrieves the byte sent by the keyboard.
    let byte = unsafe { DATA.read() };
    let event = DECODER.lock_irq_safe().advance_state(byte);
    match event {
        Ok(Some(event)) => keyboard::push_event(event),
        Ok(None) => {}
        // Acknowledgements and unknown keys are not key events.
        Err(error) => log::debug!("PS/2: ignoring byte {byte:#04x} ({error:?})"),
    }
}

/// Resets the computer by pulsing the reset line of the CPU, which is
/// connected to the controller on PC compatible hardware. Returns only if the
/// controller did not reset the computer.
pub fn pulse_reset_line() {
    if wait_input_empty().is_ok() {
        // SAFETY: Resetting the computer is the expected effect of this
        // command.
        unsafe { STATUS_COMMAND.write(COMMAND_PULSE_RESET) };
    }
}

/// Writes the configuration byte of the controller.
fn write_config(config: u8) -> Result<(), Error> {
    send_command(COMMAND_WRITE_CONFIG)?;
    write_data(config)
}

/// Sends a byte to the keyboard and waits for its acknowledgement.
fn send_to_device(byte: u8) -> Result<(), Error> {
    write_data(byte)?;
    match read_data()? {
        DEVICE_ACK => Ok(()),
        _ => Err(Error::NoAck(byte)),
    }
}

/// Sends a command to the controller.
fn send_command(command: u8) -> Result<(), Error> {
    wait_input_empty()?;
    // SAFETY: The commands sent by this driver only configure the controller.
    unsafe { STATUS_COMMAND.write(command) };
    Ok(())
}

/// Writes a byte to the data port, once the controller is ready to accept it.
fn write_data(byte: u8) -> Result<(), Error> {
    wait_input_empty()?;
    // SAFETY: The controller is ready to receive a byte.
    unsafe { DATA.write(byte) };
    Ok(())
}

/// Reads a byte from the data port, once one is available.
fn read_data() -> Result<u8, Error> {
    // SAFETY: Polling the status register has no side effect, and the data
    // port is read only once a byte is available.
    unsafe {
        match STATUS_COMMAND.poll_until(STATUS_OUTPUT_FULL, Poll::Set, TIMEOUT) {
            PollResult::Success => Ok(DATA.read()),
            _ => Err(Error::Timeout),
        }
    }
}

/// Waits until the controller has processed the last byte written to it.
fn wait_input_empty() -> Result<(), Error> {
    // SAFETY: Polling the status register has no side effect.
    match unsafe { STATUS_COMMAND.poll_until(STATUS_INPUT_FULL, Poll::Clear, TIMEOUT) } {
        PollResult::Success => Ok(()),
        _ => Err(Error::Timeout),
    }
}

/// Discards the bytes waiting in the output buffer of the controller.
fn flush() {
    for _ in 0..16 {
        // SAFETY: Reading the status register has no side effect, and the
        // data port is only read when a byte is available.
        unsafe {
            if STATUS_COMMAND.read() & STATUS_OUTPUT_FULL == 0 {
                break;
            }
            _ = DATA.read();
        }
    }
}
