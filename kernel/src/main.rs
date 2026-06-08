#![no_std]
#![no_main]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(unsafe_cell_access)]
#![feature(ptr_as_uninit)]
#![feature(step_trait)]

extern crate alloc;

use macros::init;

use crate::time::{duration::Duration, instant::Instant, timer::TimerMode};

pub mod arch;
pub mod config;
pub mod library;
pub mod mm;
pub mod time;

/// The main entry point of the kernel, common to all architectures. This
/// function is responsible for initializing the kernel subsystems and
/// starting the main loop.
///
/// # Safety
/// This function must only be called once by the architecture-specific entry
/// point after the kernel has been loaded into memory.
#[init]
pub unsafe fn main() -> ! {
    mm::page::setup();
    mm::buddy::setup();
    mm::heap::setup();

    time::timer::setup();

    log::info!("Boot completed !");

    arch::time::schedule_periodic_timer();
    arch::irq::enable();
    arch::smp::ap_run();
    idle_forever();
}

/// The idle loop of the kernel, which will be executed when there is nothing
/// else to do. This function will put the CPU to sleep until the next
/// interrupt, indefinitely.
pub fn idle_forever() -> ! {
    let cpuid = arch::smp::cpu_identifier();

    time::timer::schedule(
        TimerMode::Periodic(Duration::from_secs(2)),
        Instant::now() + Duration::from_secs(1),
        move |_| {
            log::info!("Tic (CPU {})", cpuid);
        },
    )
    .ignore();

    time::timer::schedule(
        TimerMode::Periodic(Duration::from_secs(2)),
        Instant::now() + Duration::from_secs(2),
        move |_| {
            log::info!("Tac (CPU {})", cpuid);
        },
    )
    .ignore();

    loop {
        arch::irq::wait();
    }
}
