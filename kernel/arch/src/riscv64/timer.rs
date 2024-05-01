static INTERNAL_TICK: spin::Mutex<u64> = spin::Mutex::new(0);

/// Setup the timer subsystem. It will extract the timebase frequency from the
/// device tree and calculate the internal tick value, which is the number of
/// nanoseconds per tick.
pub fn setup(device_tree: &fdt::Fdt) {
    let cpu = device_tree
        .cpus()
        .next()
        .expect("No cpu found in the device tree");

    let frequency = cpu.timebase_frequency() as u64;
    *INTERNAL_TICK.lock() = 1_000_000_000 / frequency;
}

/// Set the next timer trigger to the given duration from now. An interrupt will
/// be raised when the timer will reach the given duration.
pub fn next_trigger(next: core::time::Duration) {
    // Convert the duration to nanoseconds. It should fit in a u64 and should
    // be enough to represent the time until the next trigger.
    let nano = u64::try_from(next.as_nanos())
        .expect("Duration in nanoseconds is too large to to fit in a u64");

    // Read the current clock value and add the duration to it. It convert
    // the duration to the clock internal ticks and set the timer to the
    // new value using the SBI.
    let current = riscv::register::time::read64();
    let next = current + nano / internal_tick();
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
    *INTERNAL_TICK.lock()
}
