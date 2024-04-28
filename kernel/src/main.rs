#![no_std]
#![no_main]

pub mod boot;
pub mod config;
pub mod log;

/// The `kiwi` function is called after the architecture-specific initialization
/// was completed. It is responsible for setting up the kernel and starting the
/// first user-space process.
#[no_mangle]
pub fn kiwi() -> ! {
    #[cfg(feature = "logging")]
    log::setup();

    arch::log::write("Hello, world!\n");
    arch::shutdown();
}
