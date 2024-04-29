pub mod cpu;
pub mod log;
pub mod memory;
pub mod mmu;

/// Shutdown the system.
pub fn shutdown() -> ! {
    crate::target::shutdown();
}

/// Reboot the system.
pub fn reboot() -> ! {
    crate::target::reboot();
}
