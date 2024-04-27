/// Oops ! The kernel panicked and must be stopped. Since we are developing a
/// microkernel, this should never happen. If it does, it means that there is a
/// bug in the kernel. It will print some information about the panic if the
/// `log` feature is enabled and then stop the kernel forever.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// The entry point of the kernel. It will call architecture-specific setup
/// and then call the `kiwi` function which is the main function of the kernel
/// that will properly start the kernel and never return.
#[riscv_rt::entry]
unsafe fn _start() -> ! {
    super::setup();
    crate::kiwi()
}
