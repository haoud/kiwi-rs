use crate::{thread::Thread, trap::Resume};
use riscv::register::stvec::TrapMode;

core::arch::global_asm!(include_str!("asm/trap.asm"));

extern "C" {
    fn kernel_enter();
}

/// The context of the trap. This struct is used to store the state
/// of the CPU when the trap occured.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct Context {
    registers: [u64; 31],
    sstatus: u64,
    sepc: u64,

    /// Some padding to align the `Context` struct to 16 bytes. This
    /// is also used when executing a thread to temporarily store the
    /// kernel stack pointer into to be able to easily restore it when
    /// a trap occurs while executing the thread.
    padding: u64,
}

impl Context {
    /// Create a new context.
    pub const fn new() -> Self {
        Self {
            registers: [0; 31],
            sstatus: 0,
            sepc: 0,
            padding: 0,
        }
    }

    /// Set the stack pointer.
    pub fn set_sp(&mut self, sp: usize) {
        self.registers[1] = sp as u64;
    }

    /// Set the instruction pointer.
    pub fn set_ip(&mut self, ip: usize) {
        self.sepc = ip as u64;
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

pub fn setup() {
    // SAFETY: The function `kernel_enter` is defined in the
    // assembly file `trap.asm` and is designed to handle all
    // interrupts and exceptions.
    unsafe {
        riscv::register::stvec::write(kernel_enter as usize, TrapMode::Direct);
    }
}

pub fn handle_exception(thread: &mut Thread) -> Resume {
    let scause = riscv::register::scause::read();
    let stval = riscv::register::stval::read();
    let sepc = riscv::register::sepc::read();
    log::trace!(
        "Exception: {:?}, stval: {:#x}, sepc: {:#x}",
        scause.cause(),
        stval,
        sepc
    );
    Resume::Fault
}

pub fn handle_interrupt(thread: &mut Thread) -> Resume {
    Resume::Yield
}

#[no_mangle]
pub extern "C" fn kernel_trap_handler() {
    panic!("Kernel trap handler");
}
