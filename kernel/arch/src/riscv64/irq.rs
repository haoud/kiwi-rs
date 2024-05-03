use riscv::register::scause::{Interrupt, Trap};

core::arch::global_asm!(include_str!("asm/trap.asm"));

extern "C" {
    fn kernel_enter();
}

static BUFFER: [u128; 512] = [0; 512];

/// Setup the IRQ. It will set the `stvec` register to the address of
/// `kernel_enter` function for all interrupts, both asynchronous (IRQ)
/// and synchronous (exceptions).
pub fn setup() {
    // SAFETY: The function `kernel_enter` is defined in the assembly file
    // `trap.asm` and is designed to handle all interrupts and exceptions.
    unsafe {
        riscv::register::sscratch::write(BUFFER.as_ptr() as usize + (BUFFER.len() - 1) * 16);
        riscv::register::stvec::write(
            kernel_enter as usize,
            riscv::register::stvec::TrapMode::Direct,
        );
    }
}

/// Acknowledge the interrupt, allowing the next interrupt of the same
/// type to be delivered.
pub fn ack(_irq: u32) {}

/// Enable interrupts.
///
/// # Safety
/// This function is unsafe because it can break invariants of other code.
/// Enabling interrupts could lead to memory unsafety, race conditions,
/// deadlocks, and other undefined behavior.
pub unsafe fn enable() {
    riscv::register::sstatus::set_sie();
}

/// Disable interrupts. No interrupt will be triggered until interrupts
/// are enabled again. However, exceptions will still be triggered.
pub fn disable() {
    // SAFETY: Disabling interrupts should be safe and should
    // not cause any side effect that could lead to undefined
    // behavior.
    unsafe {
        riscv::register::sstatus::clear_sie();
    }
}

/// TEST
#[no_mangle]
pub extern "C" fn trap() {
    log::debug!("Trap handler");
    let scause = riscv::register::scause::read();
    let stval = riscv::register::stval::read();

    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // Reset the timer to avoid the interrupt to be
            // triggered again after the handler returns.
            _ = sbi::timer::set_timer(u64::MAX);
        }
        _ => {
            log::warn!(
                "Unhandled exception: {:?} (stval: {:#x})",
                scause.cause(),
                stval
            );
        }
    }
}
