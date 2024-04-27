/// Write a message to the sbi console.
pub fn write(message: &str) {
    message
        .as_bytes()
        .iter()
        .for_each(|&c| sbi::legacy::console_putchar(c));
}
