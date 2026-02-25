use crate::arch;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // If the APs are not ready, this means that the kernel panicked early
    // during boot. Therefore, we can't get the current CPU identifier because
    // per-CPU variables are not yet initialized. However, we can assume that
    // the current CPU is the bootstrap processor (BSP), which often has an
    // identifier of 0.
    let cpu_id = if arch::smp::ap_ready() {
        arch::smp::cpu_identifier()
    } else {
        0
    };

    let message = info.message();
    if let Some(location) = info.location() {
        log::error!("[CPU {}] {} at {}", cpu_id, message, location);
    } else {
        log::error!("[CPU {}] {}", cpu_id, message);
    }

    arch::cpu::freeze();
}
