#![no_std]
#![no_main]

/// The `kiwi` function is called after the architecture-specific initialization
/// was completed. It is responsible for setting up the kernel and starting the
/// first user-space process.
#[no_mangle]
pub fn kiwi() -> ! {
    arch::log::write("Hello, world!\n");
    loop {
        arch::cpu::relax();
    }
}
