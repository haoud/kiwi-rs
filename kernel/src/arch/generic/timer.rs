/// Setup the timer subsystem to raise an interrupt in the given duration from
/// the current time. If a previous timer was set, it will be replaced by this
/// new timer. If the timer was disabled by the `shutdown` function, it will be
/// automatically enabled.
pub fn next_event(next: core::time::Duration) {
    crate::arch::target::timer::next_event(next);
}

/// Shutdown the timer, preventing any further interrupts from being raised.
pub fn shutdown() {
    crate::arch::target::timer::shutdown();
}

/// The internal frequency of the timer, in Hertz. This is the frequency of the
/// timer's timebase, and is the frequency at which the timer increments.
#[must_use]
pub fn internal_frequency() -> u64 {
    crate::arch::target::timer::internal_frequency()
}

/// The duration of a single internal tick, in nanoseconds. This will be
/// the granularity of the timer, and the smallest unit of time that can
/// be measured by the timer.
#[must_use]
pub fn internal_tick() -> u64 {
    crate::arch::target::timer::internal_tick()
}
