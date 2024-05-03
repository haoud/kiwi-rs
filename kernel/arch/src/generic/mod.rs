pub mod cpu;
pub mod irq;
pub mod log;
pub mod memory;
pub mod mmu;
pub mod timer;
pub mod trap;

/// Shutdown the system.
pub fn shutdown() -> ! {
    crate::target::shutdown();
}

/// Reboot the system.
pub fn reboot() -> ! {
    crate::target::reboot();
}
