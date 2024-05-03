/// Enable IRQs.
///
/// # Safety
/// The caller must ensure that the kernel can handle IRQs correctly and that
/// enabling them is safe and will not cause any undefined behavior or memory
/// unsafety.
pub unsafe fn enable() {
    crate::target::irq::enable();
}

/// Disable IRQs.
///
/// # Safety
/// Contrary to enabling IRQs, disabling them should be safe and should not
/// cause any side effect that could lead to undefined behavior.
pub fn disable() {
    crate::target::irq::disable();
}
