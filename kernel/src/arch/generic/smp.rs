/// Get the identifier of the current CPU. The number is unique for each CPU,
/// but not necessarily sequential. The first CPU is not guaranteed to have
/// an identifier of 0.
#[must_use]
pub fn cpu_identifier() -> u8 {
    crate::arch::target::smp::cpu_identifier()
}

/// Get the total number of CPUs in the system. This is not necessarily the
/// same as the maximum CPU identifier plus one, since some CPUs may be
/// disabled or not functional.
#[must_use]
pub fn cpu_count() -> usize {
    crate::arch::target::smp::cpu_count()
}

/// Check if the application processors (APs) have finished booting and are
/// ready to be used.
#[must_use]
pub fn ap_ready() -> bool {
    crate::arch::target::smp::ap_ready()
}
