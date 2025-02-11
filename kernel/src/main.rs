#![no_std]
#![no_main]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![feature(step_trait)]
#![feature(const_option)]
#![feature(panic_info_message)]

pub mod arch;
pub mod future;
pub mod mm;
pub mod user;
pub mod utils;

extern crate alloc;

/// The initial user-space process that will be executed by the kernel. This
/// is the only user-space process that is started directly by the kernel.
/// All other user-space processes must be started by the `init` process.
static INIT: &[u8] = include_bytes!(
    "../../user/init/target/riscv64gc-unknown-none-elf/release/init"
);

/// The `kiwi` function is called after the architecture-specific
/// initialization was completed. It is responsible for setting up the
/// kernel and starting the first user-space process.
///
/// # Safety
/// This function should only be called once during the kernel boot
/// process. Once the boot process is completed, the function will
/// be wiped from memory to free up memory space.
#[macros::init]
#[no_mangle]
pub extern "Rust" fn kiwi(memory: arch::memory::UsableMemory) -> ! {
    mm::phys::setup(memory);
    mm::heap::setup();
    future::executor::setup();
    future::executor::spawn(user::elf::load(INIT));

    let memory_usage = mm::phys::kernel_memory_pages() * 4;
    log::info!("Boot completed !");
    log::info!("Memory used by the kernel: {} Kib", memory_usage);

    // Run the executor and start the first user-space process
    future::executor::run();
}
