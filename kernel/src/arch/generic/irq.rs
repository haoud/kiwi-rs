/// Enable IRQs.
///
/// # Safety
/// The caller must ensure that the kernel can handle IRQs correctly and that
/// enabling them is safe and will not cause any undefined behavior or memory
/// unsafety.
pub unsafe fn enable() {
    crate::arch::target::irq::enable();
}

/// Disable IRQs.
///
/// # Safety
/// Contrary to enabling IRQs, disabling them should be safe and should not
/// cause any side effect that could lead to undefined behavior.
pub fn disable() {
    crate::arch::target::irq::disable();
}

/// Check if IRQs are enabled.
#[must_use]
pub fn enabled() -> bool {
    crate::arch::target::irq::enabled()
}

/// Execute the given closure with IRQs disabled, returning the result of the
/// closure. If IRQs were already disabled, they will remain disabled after the
/// execution of the closure.
pub fn without<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let were_enabled = enabled();
    if were_enabled {
        disable();
    }
    let ret = f();
    if were_enabled {
        // SAFETY: We checked that IRQs were enabled before disabling them.
        // Thus, it is safe to assume that enabling them again is safe since
        // it should not cause any undefined behavior for the caller if they
        // were already enabled and working correctly.
        unsafe { enable() };
    }
    ret
}
