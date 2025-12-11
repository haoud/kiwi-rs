#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

#[unsafe(no_mangle)]
pub unsafe fn _start() -> ! {
    core::arch::asm!("ecall",
      in("a7") 1,   // syscall number for exit
      in("a0") 0,   // exit code 0
      options(noreturn)
    );
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
