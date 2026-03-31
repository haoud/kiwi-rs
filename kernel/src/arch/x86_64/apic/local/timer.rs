use macros::{init, per_cpu};

use crate::{
    arch::{x86_64::{
        self,
        apic::{self, local::Register},
    }},
    config,
    library::lock::seq::Seqlock,
};

/// The base IRQ vector for the Local APIC timer.
pub const IRQ_VECTOR: u8 = 32;

/// The initial count value for the Local APIC timer that allows to achieve a
/// fixed timer frequency. This value is computed during the calibration of 
/// the Local APIC timer, and is never modified after that.
#[per_cpu]
static INITIAL_COUNT: Seqlock<u32> = Seqlock::new(0);

/// Initialize the Local APIC timer interrupt for the current core. This
/// will configure the Local APIC timer to raise an IRQ specified by the
/// [`IRQ_VECTOR`] in one shot mode with an divide configuration of 0b0011
/// (divide by 16).
///
/// # Safety
/// The caller must ensure to only call this function once per core during
/// the initialization of the kernel, expect for the boot CPU which should
/// call [`calibrate`] instead. This function should also be called after
/// calibrating the Local APIC timer frequency with [`calibrate`].
#[init]
pub unsafe fn setup() {
    // Calibrate the Local APIC timer frequency
    calibrate();

    // Configure the Local APIC timer, respectively:
    // - Set the IRQ vector to 32, periodic mode
    // - Set the divide configuration to 0011 (divide by 16)
    // - Set the initial count value to the value computed during the
    //   calibration phase to achieve a fixed timer frequency defined
    //   by the TIMER_HZ constant in the config module.
    apic::local::write_register(Register::LVT_TIMER, u32::from(IRQ_VECTOR) | 0x20000);
    apic::local::write_register(Register::DIVIDE_CONFIGURATION, 0b0011);
    apic::local::write_register(Register::INITIAL_COUNT, INITIAL_COUNT.local().read());
}

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
    let counter = frequency / config::TIMER_HZ;
    let granularity = 1_000_000_000 / frequency;

    log::debug!(
        "(CPU {}) Local APIC timer frequency: {} Hz (granularity: {} ns)",
        x86_64::smp::cpu_identifier(),
        frequency,
        granularity
    );

    INITIAL_COUNT.local().write(counter);
}

/// Handle a Local APIC timer interrupt.
pub fn handle_irq() {
    apic::local::signal_eoi();
}
