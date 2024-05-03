use crate::trap::KERNEL_STACK;
use riscv::register::{
    scause::{Interrupt, Trap},
    stvec::TrapMode,
};

core::arch::global_asm!(include_str!("asm/trap.asm"));

/// The interrupt that caused the kernel to be interrupted. This is a
/// empty struct because the RISC-V architecture provide a `scause`
/// register that contains all the information needed to determine
/// the cause of the trap. On some architectures (e.g. x86), the
/// trap is more complex and requires a dedicated struct to store
/// all the information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptNr {}

/// The exception that caused the kernel to be interrupted. This is a
/// empty struct because the RISC-V architecture provide a `scause`
/// register that contains all the information needed to determine
/// the cause of the trap. On some architectures (e.g. x86), the
/// trap is more complex and requires a dedicated struct to store
/// all the information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionNr {}

pub fn setup() {
    extern "C" {
        fn kernel_enter();
    }

    // SAFETY: The function `kernel_enter` is defined in the assembly file
    // `trap.asm` and is designed to handle all interrupts and exceptions.
    unsafe {
        riscv::register::stvec::write(kernel_enter as usize, TrapMode::Direct);
        riscv::register::sscratch::write(KERNEL_STACK.top() as usize);
    }
}

#[no_mangle]
pub extern "C" fn trap() {
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
