/// The state of IRQs, either enabled or disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Enabled,
    Disabled,
}

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
pub fn enabled() -> State {
    if crate::arch::target::irq::enabled() {
        State::Enabled
    } else {
        State::Disabled
    }
}

/// Save the current state of IRQs and disable them. The returned state can be
/// used to restore the state of IRQs later using [`restore()`].
#[must_use]
pub fn save_and_disable() -> State {
    let state = enabled();
    disable();
    state
}

/// Restore the state of IRQs to the given state.
///
/// # Safety
/// This function is unsafe because it may enable IRQs, which can cause many,
/// many side effects that could lead to undefined behavior if the caller is
/// not careful. See the documentation of [`enable()`] for more details on the
/// safety requirements of enabling IRQs.
pub unsafe fn restore(state: State) {
    match state {
        State::Enabled => enable(),
        State::Disabled => disable(),
    }
}

/// Execute the given closure with IRQs disabled, returning the result of the
/// closure. If IRQs were already disabled, they will remain disabled after the
/// execution of the closure.
pub fn without<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let state = save_and_disable();
    let ret = f();

    // SAFETY: We checked that IRQs were enabled before disabling them.
    // Thus, it is safe to assume that enabling them again is safe since
    // it should not cause any undefined behavior for the caller if they
    // were already enabled and working correctly.
    unsafe {
        restore(state);
    };
    ret
}
