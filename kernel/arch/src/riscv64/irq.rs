core::arch::global_asm!(include_str!("asm/trap.asm"));

extern "C" {
    fn kernel_enter();
}

/// Setup the IRQ. It will set the `stvec` register to the address of
/// `kernel_enter` function for all interrupts, both asynchronous (IRQ)
/// and synchronous (exceptions).
pub fn setup() {
    // SAFETY: The function `kernel_enter` is defined in the assembly file
    // `trap.asm` and is designed to handle all interrupts and exceptions.
    unsafe {
        riscv::register::stvec::write(
            kernel_enter as usize,
            riscv::register::stvec::TrapMode::Direct,
        );
    }
}

pub fn ack(_irq: u32) {}

pub unsafe fn enable() {
    riscv::register::sie::set_stimer();
    riscv::register::sie::set_ssoft();
    riscv::register::sie::set_sext();
}

pub fn disable() {
    // SAFETY: Disabling interrupts should be safe and should not cause
    // any side effect that could lead to undefined behavior.
    unsafe {
        riscv::register::sie::clear_stimer();
        riscv::register::sie::clear_ssoft();
        riscv::register::sie::clear_sext();
    }
}

/// TEST
#[no_mangle]
pub extern "C" fn trap() {
    log::trace!("Trap");
}
