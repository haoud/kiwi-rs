use macros::init;

core::arch::global_asm!(include_str!("asm/boot.asm"));

/// Oops ! The kernel panicked and must be stopped. Since we are developing a
/// microkernel, this should never happen. If it does, it means that there is a
/// bug in the kernel. It will print some information about the panic if the
/// `log` feature is enabled and then stop the kernel forever.
#[cold]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        ::log::error!(
            "Kernel panic at {}:{}: {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        ::log::error!("Kernel panic without location or message :(");
    }

    sbi::legacy::shutdown();
}

/// The entry point of the kernel. It will call architecture-specific setup
/// and then call the `kiwi` function which is the main function of the kernel
/// that will properly start the kernel and never return.
#[init]
#[unsafe(no_mangle)]
unsafe extern "C" fn entry(hart: usize, device_tree: usize) -> ! {
    // Setup the architecture-specific stuff and start the kernel
    crate::kiwi(super::setup(hart, device_tree));
}
