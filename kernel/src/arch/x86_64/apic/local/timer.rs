use macros::{init, per_cpu};

use crate::{
    arch::{
        self,
        x86_64::{
            self,
            apic::{self, local::Register},
        },
    },
    config,
    library::lock::seq::Seqlock,
    time::{self, TimerFrequency, duration::Duration},
};

/// The base IRQ vector for the Local APIC timer.
pub const IRQ_VECTOR: u8 = 32;

/// The frequency of the Local APIC timer in Hz
#[per_cpu]
static FREQUENCY: Seqlock<u32> = Seqlock::new(0);

/// Calibrate the Local APIC timer.
///
/// This function will measure the frequency of the Local APIC timer by using the
/// PIT as a reference. It will then compute the initial count value for the
/// Local APIC timer to achieve a desired timer frequency, and store it in the
/// [`INITIAL_COUNT`] variable for later use by the [`setup`] function.
///
/// # Safety
/// This function should only be called once per core during the initialization
/// of the kernel.
#[init]
pub unsafe fn calibrate() {
    // Perform a sleep of 50 milliseconds using the PIT, and measure the
    // number of ticks elapsed in the Local APIC timer during that time.
    // We must sleep for a sufficiently long time since Kiwi is mainly used
    // inside a VM, and the Local APIC timer frequency is very high and the
    // calibration can be greatly affected by the scheduling of the VM. By
    // sleeping for a long time, we can mitigate that effect.
    let sleep_token = x86_64::pit::prepare_sleep(50);
    apic::local::write_register(Register::DIVIDE_CONFIGURATION, 0b0011);
    apic::local::write_register(Register::INITIAL_COUNT, u32::MAX);
    x86_64::pit::perform_sleep(sleep_token);

    let elapsed = u32::MAX - apic::local::read_register(Register::CURRENT_COUNT);
    let frequency = elapsed * 20;
    let granularity = 1_000_000_000 / frequency;

    log::debug!(
        "(CPU {}) Local APIC timer frequency: {} Hz (granularity: {} ns)",
        x86_64::smp::cpu_identifier(),
        frequency,
        granularity
    );

    FREQUENCY.local().write(frequency);
}

/// Set the Local APIC timer to periodic mode with the given frequency.
pub fn schedule_periodic_timer() {
    // SAFETY: Configuring the Local APIC timer in periodic mode with a valid
    // frequency should not cause any safety issues, as long as the APIC as
    // been properly initialized.
    unsafe {
        let counter = frequency_to_counter(config::TIMER_HZ);
        apic::local::write_register(Register::LVT_TIMER, u32::from(IRQ_VECTOR) | 0x20000);
        apic::local::write_register(Register::DIVIDE_CONFIGURATION, 0b0011);
        apic::local::write_register(Register::INITIAL_COUNT, counter);
    }
}

/// Get the duration elapsed since the last Local APIC timer tick.
///
/// FIXME: If this function is called after a timer tick IRQ has been triggered
/// but before the timer tick handler has been called, it will return the
/// duration elapsed since the last triggered timer tick, which is not up to
/// date with the current timer tick of the kernel.
/// Possible solutions to this issue include:
/// - Using the oneshot mode in order to avoid the internal counter of the Local
///   APIC timer to restart before the timer tick handler is called. This will
///   increase the overhead a little bit and may drift the timer frequency.
/// - Using a separate counter to track the elapsed time since the last timer
///   tick, for example by using the TSC if an invariant TSC is available.
#[must_use]
pub fn since_last_tick() -> Duration {
    let apic_frequency = u64::from(FREQUENCY.local().read());
    if apic_frequency == 0 {
        return Duration::from_nanos(0);
    }

    let initial = frequency_to_counter(config::TIMER_HZ);
    let elapsed = initial - read_current_count();
    let elapsed = (u64::from(elapsed) * 1_000_000_000) / apic_frequency;
    Duration::from_nanos(elapsed)
}

/// Convert a timer frequency in Hz to a initial counter value for the
/// Local APIC timer.
fn frequency_to_counter(frequency: TimerFrequency) -> u32 {
    FREQUENCY.local().read() / frequency.as_hertz()
}

/// Read the current count value of the Local APIC timer.
#[must_use]
fn read_current_count() -> u32 {
    // SAFETY: This function is only reading from the Local APIC timer's
    // current count register and should be safe to call at any time after
    // the Local APIC timer has been initialized.
    unsafe { apic::local::read_register(Register::CURRENT_COUNT) }
}

/// Handle a Local APIC timer interrupt.
pub fn handle_irq() {
    apic::local::signal_eoi();
    if arch::smp::is_bsp() {
        time::jiffies::increment_jiffies();
        time::timer::handle_expired_timers();
    }
}
