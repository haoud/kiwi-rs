use crate::generic;

pub mod cpu;
pub mod log;

mod lang;

/// Setup the riscv64 architecture
pub fn setup() {
    #[cfg(feature = "logging")]
    generic::log::setup();
}

/// Shutdown the computer
#[inline]
pub fn shutdown() -> ! {
    sbi::legacy::shutdown()
}

/// Reboot the computer. If for some reason the SBI call fails, we will just
/// perform a shutdown instead.
#[inline]
pub fn reboot() -> ! {
    _ = sbi::system_reset::system_reset(
        sbi::system_reset::ResetType::ColdReboot,
        sbi::system_reset::ResetReason::NoReason,
    );
    sbi::legacy::shutdown()
}
