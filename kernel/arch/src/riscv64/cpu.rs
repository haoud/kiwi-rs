/// Relaxes the CPU by waiting for an interrupt.
#[inline]
pub fn relax() {
    unsafe {
        core::arch::asm!("wfi");
    }
}
