/// Acknowledge an exception, allowing the CPU to service the next one.
pub fn ack(exception: u32) {
    crate::target::exception::ack(exception);
}
