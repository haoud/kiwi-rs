use crate::{arch::x86_64, time::duration::Duration};

/// Setup a periodic local timer event to occur at the frequency given by
/// [`config::TIMER_HZ`]. This will configure the local timer to automatically
/// re-schedule itself after each timer event, allowing to have a periodic
/// timer event without having to manually schedule the next event in the
/// timer interrupt handler.
pub fn schedule_periodic_timer() {
    x86_64::apic::local::timer::schedule_periodic_timer();
}

/// Returns the duration since the last timer tick, which can be used to
/// estimate the time until the next timer tick. This is useful for
/// implementing functions like `Instant::now()` that need to provide a
/// high-resolution time measurement that is not limited to the granularity
/// of the timer ticks.
#[must_use]
pub fn since_last_tick() -> Duration {
    x86_64::apic::local::timer::since_last_tick()
}
