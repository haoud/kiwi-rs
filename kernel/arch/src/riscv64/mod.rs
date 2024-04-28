use crate::generic;

pub mod cpu;
pub mod log;

mod lang;

/// Setup the riscv64 architecture
pub fn setup(hart: usize, device_tree: *const u8) {
    #[cfg(feature = "logging")]
    generic::log::setup();

    ::log::trace!("Booting on hart {hart}");
    ::log::trace!("Device tree is at {:p}", device_tree);

    // Parse the device tree using the `fdt` crate
    // SAFETY: We must assume that the device tree pointer is valid
    let fdt = unsafe {
        fdt::Fdt::from_ptr(device_tree)
            .expect("Failed to parse the device tree : cannot continue without this !")
    };

    // Information about memory regions
    for region in fdt.memory().regions() {
        ::log::trace!(
            "Memory region: {:#x} - {:#x}",
            region.starting_address as usize,
            region.starting_address as usize + region.size.unwrap_or(0)
        );
    }

    // Information about reserved memory regions
    for region in fdt.memory_reservations() {
        ::log::trace!(
            "Memory region: {:#x} - {:#x}",
            region.address() as usize,
            region.address() as usize + region.size()
        );
    }
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
