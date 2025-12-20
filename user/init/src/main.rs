#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

struct KernelString {
    data: *const u8,
    len: usize,
}

pub fn register_as_service(name: &str) {
    let kstr = KernelString {
        data: name.as_ptr(),
        len: name.len(),
    };

    unsafe {
        core::arch::asm!("ecall",
          in("a7") 2,               // syscall number for service_register
          in("a0") kstr.data,       // pointer to the service name
          in("a1") kstr.len,        // length of the service name
        );
    }
}

pub fn unregister_as_service() {
    unsafe {
        core::arch::asm!("ecall",
          in("a7") 3,       // syscall number for service_unregister
        );
    }
}

pub fn exit(code: i32) -> ! {
    unsafe {
        core::arch::asm!("ecall",
          in("a7") 1,       // syscall number for exit
          in("a0") code,    // exit code
          options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe fn _start() -> ! {
    register_as_service("init");
    unregister_as_service();
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
