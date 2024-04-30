use crate::{generic, memory::UsableMemory};

pub mod cpu;
pub mod log;
pub mod memory;
pub mod mmu;

mod lang;

/// Setup the riscv64 architecture
pub fn setup(hart: usize, device_tree: *const u8) {
    #[cfg(feature = "logging")]
    generic::log::setup();

    // Some debug informations
    ::log::trace!("Booting on hart {hart}");
    ::log::trace!("Device tree is at {:p}", device_tree);

    // Initialize the MMU
    mmu::setup();
    ::log::info!("MMU initialized");

    // Parse the device tree using the `fdt` crate
    // SAFETY: We must assume that the device tree pointer is valid
    let fdt = unsafe {
        fdt::Fdt::from_ptr(device_tree)
            .expect("Failed to parse the device tree : cannot continue without this !")
    };

    let memory = UsableMemory::new(&fdt);
    ::log::info!("Usable memory count: {} kio", memory.size() / 1024);
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
