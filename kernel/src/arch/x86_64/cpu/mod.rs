use crate::arch::x86_64::{self, cpu};

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
    pub rbp: Register,
    pub rbx: Register,
    pub r12: Register,
    pub r13: Register,
    pub r14: Register,
    pub r15: Register,

    // Scratched registers
    pub rax: Register,
    pub rcx: Register,
    pub rdx: Register,
    pub rsi: Register,
    pub rdi: Register,
    pub r8: Register,
    pub r9: Register,
    pub r10: Register,
    pub r11: Register,

    /// Custom data pushed by the interrupt handler. This data is used to pass
    /// additional information to the interrupt handler. For example, the IRQ
    /// number for an interrupt is pushed in this field.
    pub data: u64,

    /// The error code. It is either pushed by the CPU automatically when
    /// certain exceptions are triggered or pushed by the interrupt handler.
    /// In the last case, the error code is set to 0.
    pub error: u64,

    // Pushed by the CPU automatically when an interrupt is triggered
    pub rip: Register,
    pub cs: Register,
    pub rflags: cpu::rflags::Flags,
    pub rsp: Register,
    pub ss: Register,
}

/// Represents a general-purpose register in the CPU that encapsulates a 64-bit
/// unsigned integer value and provides methods for creating and manipulating
/// register values more easily.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Register(pub u64);

impl Register {
    /// Represents a register with a value of zero. This is useful for
    /// initializing registers or for operations that require a zero value.
    pub const ZERO: Register = Register(0);

    /// Creates a new `Register` with the given value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Register(value)
    }
}

impl From<u16> for Register {
    fn from(value: u16) -> Self {
        Register(u64::from(value))
    }
}

impl From<u32> for Register {
    fn from(value: u32) -> Self {
        Register(u64::from(value))
    }
}

impl From<u64> for Register {
    fn from(value: u64) -> Self {
        Register(value)
    }
}

#[cfg(target_pointer_width = "64")]
impl From<usize> for Register {
    fn from(value: usize) -> Self {
        Register(value as u64)
    }
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
