use crate::arch::x86_64;

/// The different privilege levels of the CPU. Kiwi only use kernel (ring 0)
/// and user (ring 3) privilege levels. Other privilege levels (ring 1 and
/// ring 2) are not used by Kiwi as well as most operating systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Privilege {
    Kernel = 0,
    User = 3,
}

/// Halt the current CPU core forever. This function will stop the CPU
/// core and will not return. This function is useful when the kernel
/// encounters a critical error and cannot recover from it.
pub fn freeze() -> ! {
    loop {
        // SAFETY: This is safe because this halt the CPU until the next
        // reboot.. And safety is not a concern at this point ;)
        unsafe {
            x86_64::instr::cli();
            x86_64::instr::hlt();
        }
    }
}
