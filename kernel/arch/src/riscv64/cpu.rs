/// Relaxes the CPU by waiting for an interrupt. This function use the `wfi` instruction
/// to wait for an interrupt and give an hint to the CPU that it can enter a low power
/// state. However, the caller should not rely on this function to put the CPU in a low
/// power state, as this instruction can be implemented as a no-op on some platforms
/// according to the RISC-V specification.
#[inline]
pub fn relax() {
    unsafe {
        core::arch::asm!("wfi");
    }
}
