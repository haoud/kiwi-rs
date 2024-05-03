#![no_std]
#![no_main]

pub mod task;

/// The `kiwi` function is called after the architecture-specific initialization
/// was completed. It is responsible for setting up the kernel and starting the
/// first user-space process.
///
/// # Safety
/// This function should only be called once during the kernel boot process. Once
/// the boot process is completed, the function will be wiped from memory to free
/// up memory space.
#[macros::init]
#[no_mangle]
pub fn kiwi() -> ! {
    arch::log::write("Hello, world!\n");
    unsafe {
        arch::irq::enable();
    }
    event_loop();
}

#[inline(never)]
pub fn event_loop() -> ! {
    loop {
        arch::timer::next_event(core::time::Duration::from_secs(1));
        arch::cpu::relax();
    }
}
