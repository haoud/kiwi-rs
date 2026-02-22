use macros::init;

use crate::arch;

pub mod boot;
pub mod cpu;
pub mod instr;
pub mod irq;
pub mod lang;
pub mod logging;
pub mod msr;
pub mod page;
pub mod percpu;
pub mod smp;

/// The entry point of the kernel.
///
/// # Safety
/// This function does rely on nothing and must initialize the kernel. Of course
/// this is highly unsafe ! This function will rely on black magic to properly
/// initialize the kernel and setup an environment more developer-friendly.
#[init]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn start() -> ! {
    arch::log::setup();
    boot::setup();
    arch::percpu::setup();
    smp::setup();

    log::info!("Boot completed !");
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
#[init]
unsafe extern "C" fn ap_start(cpu: &limine::mp::Cpu) -> ! {
    log::debug!("CPU {} is starting...", cpu.id);
    let cpu_id = cpu
        .id
        .try_into()
        .expect("CPU ID is too large to fit into a u8");

    arch::percpu::setup();
    smp::ap_setup(cpu_id);

    log::debug!(
        "CPU {} has completed its setup !",
        arch::smp::cpu_identifier()
    );
    arch::cpu::freeze();
}
