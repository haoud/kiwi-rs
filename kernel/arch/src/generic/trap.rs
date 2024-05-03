use crate::target::trap::{ExceptionNr, InterruptNr};

pub static KERNEL_STACK: KernelStack = KernelStack::new();

/// The kernel stack. This is used to handle interrupts and exceptions.
/// The stakc is packed inside a struct to ensure that it is properly
/// aligned to 16 bytes, which sould be enough for most architectures.
#[repr(align(16))]
pub struct KernelStack {
    stack: [u8; config::KERNEL_STACK_SIZE],
}

impl KernelStack {
    /// Create a new kernel stack.
    #[must_use]
    pub const fn new() -> Self {
        Self { stack: [0; 4096] }
    }

    /// Get the bottom of the stack.
    #[must_use]
    pub fn bottom(&self) -> *const u8 {
        self.stack.as_ptr()
    }

    /// Get the top of the stack.
    #[must_use]
    pub fn top(&self) -> *const u8 {
        self.bottom().wrapping_add(self.stack.len())
    }
}

/// The trap that caused the kernel to be interrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    /// An interrupt, which is an asynchronous event. It is often
    /// triggered by a peripheral device, such as a timer, a network
    /// card, a keyboard...
    Interrupt(InterruptNr),

    /// An exception, which is a synchronous event triggered by the CPU
    /// when it encounters an error or an unexpected condition while
    /// executing an instruction.
    Exception(ExceptionNr),

    /// A syscall, which is a synchronous event directly triggered
    /// by the user-space application.
    Syscall(Syscall),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Syscall {
    /// No operation syscall, used for testing purposes.
    Nop = 0,
}
