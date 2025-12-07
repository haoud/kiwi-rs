#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

#[unsafe(no_mangle)]
pub unsafe fn _start() -> ! {
    core::arch::asm!("ecall");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
