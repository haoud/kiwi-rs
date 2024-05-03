/// Acknowledge the interrupt, allowing the next interrupt of the same
/// type to be delivered.
pub fn ack(_irq: u32) {}

/// Enable interrupts.
///
/// # Safety
/// This function is unsafe because it can break invariants of other code.
/// Enabling interrupts could lead to memory unsafety, race conditions,
/// deadlocks, and other undefined behavior.
pub unsafe fn enable() {
    riscv::register::sstatus::set_sie();
}

/// Disable interrupts. No interrupt will be triggered until interrupts
/// are enabled again. However, exceptions will still be triggered.
pub fn disable() {
    // SAFETY: Disabling interrupts should be safe and should
    // not cause any side effect that could lead to undefined
    // behavior.
    unsafe {
        riscv::register::sstatus::clear_sie();
    }
}
