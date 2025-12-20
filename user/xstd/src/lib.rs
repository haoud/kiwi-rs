#![no_std]

/// Re-export the main macro
pub use macros::main;

pub mod ipc;
pub mod service;
pub mod syscall;
pub mod task;

/// The panic handler for user-space applications. When a panic occurs, this
/// function will be called, and it will simply abort the current task by
/// exiting with a non-zero exit code. Aborting the task is a simple way to
/// handle panics in user-space applications, but will not provide any
/// debugging information and will not run destructors for any remaining
/// objects.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    task::exit(-1)
}
