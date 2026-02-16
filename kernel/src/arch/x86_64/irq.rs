use crate::arch::x86_64;

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
    let rflags: u64;
    // SAFETY: Reading the RFLAGS register should not cause any memory unsafety
    // or any unexpected side effects. We ensure that the stack pointer remains
    // unchanged by pushing the flags onto the stack and popping them into a
    // register.
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem, preserves_flags)
        );
    }
    rflags & (1 << 9) != 0
}
