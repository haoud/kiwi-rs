/// Write a message to the log. This function is only by the internal logging
/// functions, only included if the `log` feature is enabled. On most platforms,*
/// this function will write to the serial port or the console.
pub fn write(message: &str) {
    crate::target::log::write(message);
}
