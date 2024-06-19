//! TODO:
//!     - Clippy lints
//!     - Monolithic kernel
//!         * Paging code can directly allocate memory
//!         * Reserve some memory for the kernel to avoid panicking
//!     - Executor than can mix user and kernel task
//!     - Proper async suspend: If this is an user task that is suspended,
//!     save it quantum and restore it when resumed
//!     - Reduce memory usage for the physical memory management
//!     - Request memory used during the boot process
//!     - Use structure instead of (too) many arguments for some functions
//!     - Create an addr crate to handle addresses
//!     - Move the `arch` crate to the `kernel` crate
#![no_std]
#![no_main]

pub mod elf;
pub mod future;
pub mod heap;
pub mod pmm;
pub mod process;

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
pub fn kiwi(memory: arch::memory::UsableMemory) -> ! {
    pmm::setup(memory);
    heap::setup();
    future::executor::setup();
    future::executor::spawn(elf::load(INIT));

    let memory_usage = pmm::kernel_memory_pages() * 4;
    log::info!("Boot completed !");
    log::info!("Memory used by the kernel: {} Kib", memory_usage);

    // Run the executor and start the first user-space process
    future::executor::run();
}
