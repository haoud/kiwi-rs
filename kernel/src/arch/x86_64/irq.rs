use crate::arch::x86_64::{self, cpu::rflags};

/// Wait for the next IRQ by halting the CPU until an interrupt is received.
///
/// If interrupts are disabled, this function will halt indefinitely until a
/// non-maskable interrupt (NMI) is received.
pub fn wait() {
    x86_64::instr::hlt();
}

/// Enable IRQs.
///
/// # Safety
/// This function is unsafe because enabling interrupts can lead to memory
/// unsafety if the interrupts are not properly handled.
pub unsafe fn enable() {
    x86_64::instr::sti();
}

/// Disable IRQs.
pub fn disable() {
    // SAFETY: Disabling interrupts shouldn't not cause any memory unsafety
    // (on the contrary, it usually helps to avoid them!) or any unexpected
    // side effects.
    unsafe {
        x86_64::instr::cli();
    }
}

/// Check if IRQs are enabled on the current core.
#[must_use]
pub fn enabled() -> bool {
    x86_64::cpu::rflags::read().contains(rflags::Flags::IF)
}
