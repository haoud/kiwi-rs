use crate::arch::{generic, memory::UsableMemory};
use macros::init;

pub mod addr;
pub mod cpu;
pub mod irq;
pub mod log;
pub mod memory;
pub mod mmu;
pub mod thread;
pub mod timer;
pub mod trap;

mod lang;

/// Setup the riscv64 architecture
///
/// # Safety
/// This function is unsafe because this function should only be called once
/// and during the boot process. During this time, we must assume some
/// invariants that we cannot guarantee in safe code.
///
/// # Panics
/// This function panics if the device tree is invalid or if the memory
/// regions are invalid.
#[must_use]
#[init]
pub fn setup(hart: usize, device_tree: *const u8) -> UsableMemory {
    #[cfg(feature = "logging")]
    generic::log::setup();

    ::log::info!("Booting the riscv64 kernel");
    ::log::info!("Hello world ! Booting on hart {}", hart);

    // Parse the device tree using the `fdt` crate
    // SAFETY: We must assume that the device tree pointer is valid
    let fdt = unsafe { fdt::Fdt::from_ptr(device_tree).expect("Failed to parse the device tree") };
    let memory = UsableMemory::new(&fdt);

    mmu::setup();
    trap::setup();
    timer::setup(&fdt);

    memory
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
    ::log::info!("Rebooting the computer");
    _ = sbi::system_reset::system_reset(
        sbi::system_reset::ResetType::ColdReboot,
        sbi::system_reset::ResetReason::NoReason,
    );
    ::log::warn!("Failed to reboot the computer, trying to shutdown instead");
    sbi::legacy::shutdown()
}
