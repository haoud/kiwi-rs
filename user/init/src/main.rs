#![no_std]
#![no_main]

#[no_mangle]
pub unsafe fn _start() -> ! {
    core::arch::asm!("ecall");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
