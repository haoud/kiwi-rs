pub mod cpu;
pub mod irq;
pub mod log;
pub mod memory;
pub mod mmu;
pub mod thread;
pub mod timer;
pub mod trap;

/// Shutdown the system.
pub fn shutdown() -> ! {
    crate::arch::target::shutdown();
}

/// Reboot the system.
pub fn reboot() -> ! {
    crate::arch::target::reboot();
}
