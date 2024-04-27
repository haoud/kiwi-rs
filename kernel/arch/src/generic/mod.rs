pub mod cpu;
pub mod log;

/// Shutdown the system.
pub fn shutdown() -> ! {
    crate::target::shutdown();
}

/// Reboot the system.
pub fn reboot() -> ! {
    crate::target::reboot();
}
