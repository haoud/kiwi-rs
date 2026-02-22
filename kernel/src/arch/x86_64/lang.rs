use crate::arch;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let cpu_id = arch::smp::cpu_identifier();
    let message = info.message();
    if let Some(location) = info.location() {
        log::error!("[CPU {}] {} at {}", cpu_id, message, location);
    } else {
        log::error!("[CPU {}] {}", cpu_id, message);
    }

    arch::cpu::freeze();
}
