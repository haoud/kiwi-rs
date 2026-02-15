use crate::arch::x86_64;

/// Write a message to the log. This function is only by the internal
/// logging functions, only included if the `log` feature is enabled.
/// On most platforms, this function will write to the serial port or
/// the console.
pub fn write(message: &str) {
    // SAFETY: Use the serial port to write the message should be safe
    // even if the serial port was not initialized.
    message.as_bytes().iter().for_each(|&byte| unsafe {
        x86_64::instr::outb(0x3F8, byte);
        x86_64::instr::outb(0x80, 0);
    });
}
