#![no_std]
#![no_main]

pub mod elf;
pub mod heap;
pub mod pmm;

/// The initial user-space process that will be executed by the kernel. This is the
/// only user-space process that is started directly by the kernel. All other user-space
/// processes must be started by the `init` process.
static INIT: &[u8] = include_bytes!(
    "../../user/init/target/riscv64gc-unknown-none-elf/release/init"
);

/// The `kiwi` function is called after the architecture-specific initialization
/// was completed. It is responsible for setting up the kernel and starting the
/// first user-space process.
///
/// # Safety
/// This function should only be called once during the kernel boot process. Once
/// the boot process is completed, the function will be wiped from memory to free
/// up memory space.
#[macros::init]
#[no_mangle]
pub fn kiwi(memory: arch::memory::UsableMemory) -> ! {
    pmm::setup(memory);
    //heap::setup();

    arch::thread::execute(&mut elf::load(INIT));
    log::debug!("Thread trapped back to kernel");
    arch::cpu::freeze();
}
