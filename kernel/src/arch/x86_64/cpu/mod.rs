use crate::arch::x86_64;

pub mod cpuid;
pub mod cr0;
pub mod cr2;
pub mod cr3;
pub mod cr4;
pub mod rflags;

/// The stack frame of an interrupt handler. This structure is used to store
/// the state of the CPU when an interrupt occurs, and it is passed to the 
/// interrupt handler as an argument.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct InterruptFrame {
    // Preserved registers
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Scratched registers
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,

    /// Custom data pushed by the interrupt handler. This data is used to pass
    /// additional information to the interrupt handler. For example, the IRQ
    /// number for an interrupt is pushed in this field.
    pub data: u64,

    /// The error code. It is either pushed by the CPU automatically when
    /// certain exceptions are triggered or pushed by the interrupt handler.
    /// In the last case, the error code is set to 0.
    pub error: u64,

    // Pushed by the CPU automatically when an interrupt is triggered
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// The different privilege levels of the CPU. Kiwi only use kernel (ring 0)
/// and user (ring 3) privilege levels. Other privilege levels (ring 1 and
/// ring 2) are not used by Kiwi as well as most operating systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Privilege {
    Kernel = 0,
    User = 3,
}

/// Halt the current CPU core forever. This function will stop the CPU
/// core and will not return. This function is useful when the kernel
/// encounters a critical error and cannot recover from it.
pub fn freeze() -> ! {
    loop {
        // SAFETY: This is safe because this halt the CPU until the next
        // reboot.. And safety is not a concern at this point ;)
        unsafe {
            x86_64::instr::cli();
            x86_64::instr::hlt();
        }
    }
}
