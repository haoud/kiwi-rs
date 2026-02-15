/// Halt the current CPU core forever. This function will stop the CPU
/// core and will not return. This function is useful when the kernel
/// encounters a critical error and cannot recover from it.
pub fn freeze() -> ! {
    crate::arch::target::cpu::freeze()
}
