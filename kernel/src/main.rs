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

pub mod arch;
pub mod config;
pub mod library;
pub mod mm;
pub mod scheduler;
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
    scheduler::setup();

    arch::time::schedule_periodic_timer();
    arch::irq::enable();
    arch::smp::ap_run();

    log::info!("Boot completed !");

    scheduler::spawn(|| log::info!("Hello from thread A"));
    scheduler::spawn(|| log::info!("Hello from thread B"));
    scheduler::run();
}
