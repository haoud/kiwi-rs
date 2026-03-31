use macros::init;

pub mod io;
pub mod local;

/// Setup the APIC (Advanced Programmable Interrupt Controller).
///
/// # Safety
/// This function should only be called once, and only during the
/// kernel initialization phase.
#[init]
pub unsafe fn setup() {
    // TODO: Remap the APIC MMIO without caching
}
