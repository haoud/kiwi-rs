#![no_std]
#![no_main]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(unsafe_cell_access)]

use macros::init;

pub mod arch;
pub mod config;
pub mod library;
pub mod mm;

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

    log::info!("Boot completed !");
    arch::cpu::freeze();
}
