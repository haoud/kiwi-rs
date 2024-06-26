use seqlock::Seqlock;

/// The internal timer frequency, in Hertz. This is the rate at which the timer
/// counter is incremented/decremented.
static INTERNAL_TICK: Seqlock<u64> = Seqlock::new(0);

/// Setup the timer subsystem. It will extract the timebase frequency from the
/// device tree and calculate the internal tick value, which is the number of
/// nanoseconds per tick.
///
/// # Panics
/// Panics if no CPU information is found in the device tree.
pub fn setup(device_tree: &fdt::Fdt) {
    log::info!("Initializing timer");

    let cpu = device_tree
        .cpus()
        .next()
        .expect("No cpu found in the device tree");

    let frequency = cpu.timebase_frequency() as u64;
    INTERNAL_TICK.write(1_000_000_000 / frequency);

    log::debug!("Internal timer tick: {} ns", internal_tick());
    log::debug!("Internal timer frequency: {} Hz", internal_frequency());

    // Enable timer interrupts.
    unsafe {
        riscv::register::sie::set_stimer();
    }
}

/// Shutdown the timer, preventing any further interrupts from being raised.
pub fn shutdown() {
    _ = sbi::timer::set_timer(u64::MAX);
}

/// Set the next timer trigger to the given duration from now. An interrupt
/// will be raised when the timer will reach the given duration.
///
/// # Panics
/// Panics if the SBI call to set the timer fails.
pub fn next_event(next: core::time::Duration) {
    let secs = next.as_secs() * 1_000_000_000;
    let nanos = u64::from(next.subsec_nanos());

    // Read the current clock value and add the duration to it.
    // It convert the duration to the clock internal ticks and
    // set the timer to the new value using the SBI.
    let current = riscv::register::time::read64();
    let next = current + (secs + nanos) / internal_tick();
    sbi::timer::set_timer(next).unwrap();
}

/// The internal frequency of the timer, in Hertz.
#[must_use]
pub fn internal_frequency() -> u64 {
    1_000_000_000 / internal_tick()
}

/// The duration of a single internal tick, in nanoseconds.
#[must_use]
pub fn internal_tick() -> u64 {
    INTERNAL_TICK.read()
}
