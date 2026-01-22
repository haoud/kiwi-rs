#![no_std]
#![no_main]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![feature(strict_provenance_lints)]
#![warn(fuzzy_provenance_casts)]
#![warn(lossy_provenance_casts)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::module_name_repetitions)]

pub mod arch;
pub mod config;
pub mod future;
pub mod ipc;
pub mod mm;
pub mod time;
pub mod user;
pub mod utils;

extern crate alloc;

/// The initial user-space process that will be executed by the kernel. This
/// is the only user-space process that is started directly by the kernel.
/// All other user-space processes must be started by the `init` process.
///
/// # Why using `include_bytes!` this way?
/// This approach ensures that the binary is included in the kernel binary
/// at compile time, and is placed in a specific section of the kernel's
/// memory layout designated for initialization data. Using the `#[macros::initdata]`
/// attribute on an &[u8] only put the reference to the data in that section,
/// not the data itself.
#[macros::initdata]
static INIT: [u8; include_bytes!(
    "../../user/init/target/riscv64gc-unknown-none-elf/release/init"
)
.len()] = *include_bytes!("../../user/init/target/riscv64gc-unknown-none-elf/release/init");

/// The echo user-space process binary. This process is used to demonstrate
/// inter-process communication (IPC) capabilities of the kernel, and is not
/// meant to stay here permanently and will be removed in future versions.
#[macros::initdata]
static ECHO: [u8; include_bytes!(
    "../../user/echo/target/riscv64gc-unknown-none-elf/release/echo"
)
.len()] = *include_bytes!("../../user/echo/target/riscv64gc-unknown-none-elf/release/echo");

/// The `kiwi` function is called after the architecture-specific
/// initialization was completed. It is responsible for setting up the
/// kernel and starting the first user-space process.
///
/// # Safety
/// This function should only be called once during the kernel boot
/// process. Once the boot process is completed, the function will
/// be wiped from memory to free up memory space.
#[macros::init]
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn kiwi(memory: arch::memory::UsableMemory) -> ! {
    mm::phys::setup(memory);
    mm::heap::setup();
    future::executor::setup();
    future::executor::spawn(user::elf::load(&INIT));
    future::executor::spawn(user::elf::load(&ECHO));

    ipc::service::setup();

    let memory_usage = mm::phys::kernel_memory_pages() * 4;
    log::info!("Boot completed !");
    log::info!("Memory used by the kernel: {} Kib", memory_usage);

    // Run the executor and start the first user-space process
    future::executor::run();
}
