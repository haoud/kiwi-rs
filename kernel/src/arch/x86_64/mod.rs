use crate::arch;

pub mod cpu;
pub mod instr;
pub mod irq;
pub mod lang;
pub mod logging;
pub mod smp;

/// The entry point of the kernel.
///
/// # Safety
/// This function does rely on nothing and must initialize the kernel. Of course
/// this is highly unsafe ! This function will rely on black magic to properly
/// initialize the kernel and setup an environment more developper-friendly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn start() -> ! {
    arch::log::setup();
    smp::setup();

    log::info!("Welcome to Kiwi!");
    arch::cpu::freeze();
}

/// The entry point of the application processors (APs).
///
/// This is a bit different from the `start` function as it will be called by
/// the APs after they have been started by the BSP. This function will be
/// responsible for initializing the APs, but will not do much else as the BSP
/// will be responsible for doing the heavy lifting of initializing the kernel
/// and setting up the environment.
///
/// # Safety
/// This function should only be called by the APs after they have been started
/// by the BSP and should not be called by any other code. In short: don't use
/// this function.
unsafe extern "C" fn ap_start(cpu: &limine::mp::Cpu) -> ! {
    log::debug!("CPU {} is starting...", cpu.id);
    smp::ap_setup();
    arch::cpu::freeze();
}
