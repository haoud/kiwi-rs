#![no_std]
#![no_main]

/// The `kiwi` function is called after the architecture-specific initialization
/// was completed. It is responsible for setting up the kernel and starting the
/// first user-space process.
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
