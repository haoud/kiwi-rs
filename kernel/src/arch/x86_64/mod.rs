use crate::arch;

pub mod cpu;
pub mod instr;
pub mod irq;
pub mod lang;
pub mod logging;

/// The entry point of the kernel.
///
/// # Safety
/// This function does rely on nothing and must initialize the kernel. Of course
/// this is highly unsafe ! This function will rely on black magic to properly
/// initialize the kernel and setup an environment more developper-friendly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn start() -> ! {
    arch::log::setup();
    log::info!("Welcome to Kiwi!");
    arch::cpu::freeze();
}
