pub mod cpu;
pub mod log;
pub mod memory;
pub mod mmu;
pub mod timer;

/// Shutdown the system.
pub fn shutdown() -> ! {
    crate::target::shutdown();
}

/// Reboot the system.
pub fn reboot() -> ! {
    crate::target::reboot();
}
