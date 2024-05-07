use super::log;
use macros::init;

core::arch::global_asm!(include_str!("asm/boot.asm"));

extern "C" {
    static __bss_start: [u8; 0];
    static __bss_end: [u8; 0];
}

/// Oops ! The kernel panicked and must be stopped. Since we are developing a
/// microkernel, this should never happen. If it does, it means that there is a
/// bug in the kernel. It will print some information about the panic if the
/// `log` feature is enabled and then stop the kernel forever.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(message) = info.payload().downcast_ref::<&str>() {
        log::write("Kernel panic: ");
        log::write(message);
        log::write(", exiting :(\n");
    } else {
        log::write("Kernel panic, exiting :(\n");
    }
    sbi::legacy::shutdown();
}

/// The entry point of the kernel. It will call architecture-specific setup
/// and then call the `kiwi` function which is the main function of the kernel
/// that will properly start the kernel and never return.
#[init]
#[no_mangle]
unsafe extern "C" fn entry(hart: usize, device_tree: usize) -> ! {
    // Clear the BSS section
    let bss_start = core::ptr::addr_of!(__bss_start) as usize;
    let bss_end = core::ptr::addr_of!(__bss_end) as usize;
    let bss_size = bss_end - bss_start;
    core::ptr::write_bytes(core::ptr::addr_of!(__bss_start) as *mut u8, 0, bss_size);

    // Setup the architecture-specific stuff and start the kernel
    crate::kiwi(super::setup(hart, device_tree as *const u8));
}
