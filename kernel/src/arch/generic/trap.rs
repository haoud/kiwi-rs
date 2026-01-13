use crate::config;

/// The stack used by the kernel to handle interrupts and exceptions. Kiwi
/// has made the choice to use a single stack per core to handle interrupts
/// instead of using a separate kernel stack for threads.
/// This brings several advantages:
///     - Less memory usage, can save dozens of megabytes of memory
///       on a system with many threads.
///     - Better cache locality, as the kernel stack will have more
///       chances to be in the cache.
///     - Simpler code
/// The main inconvenience of this approach is that the kernel cannot be
/// preemted during the middle of its execution. However, this is less of
/// a problem for a microkernel like Kiwi, as most of the kernel code should
/// be simple and fast to execute. We can also use cooperative preemption by
/// leveraging the async/await feature of Rust to reduce the preemption latency
/// by allowing the kernel to yield the CPU to another thread when it is
/// waiting for an event.
pub static KERNEL_STACK: KernelStack = KernelStack::new();

/// The kernel stack. This is used to handle interrupts and exceptions.
/// The stack is packed inside a struct to ensure that it is properly
/// aligned to 16 bytes, which should be enough for most architectures.
#[repr(align(16))]
pub struct KernelStack {
    stack: [u8; config::KERNEL_STACK_SIZE],
}

impl KernelStack {
    /// Create a new kernel stack.
    ///
    /// # Usage
    /// This function should only be used to create static instances of
    /// `KernelStack`, like the `KERNEL_STACK` static variable defined
    /// above. Calling this function at runtime will overflow the stack
    /// since Rust does not support placement allocation !
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stack: [0; config::KERNEL_STACK_SIZE],
        }
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

impl Default for KernelStack {
    fn default() -> Self {
        Self::new()
    }
}

/// The trap that caused the kernel to be interrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    /// An interrupt, which is an asynchronous event. It is often
    /// triggered by a peripheral device, such as a timer, a network
    /// card, a keyboard...
    Interrupt,

    /// An exception, which is a synchronous event triggered by the CPU
    /// when it encounters an error or an unexpected condition while
    /// executing an instruction.
    Exception,

    /// A syscall, which is a synchronous event directly triggered
    /// by the user-space application.
    Syscall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resume {
    /// Continue the execution of the thread where it was interrupted.
    Continue,

    /// Terminate the execution of the thread. This is used when the
    /// thread has finished its execution with an exit syscall.
    Terminate(i32),

    /// Yield the CPU to another thread.
    Yield,

    /// The thread has encountered a fault and should be terminated.
    Fault,
}

pub fn handle_exception(thread: &mut crate::arch::thread::Thread) -> Resume {
    crate::arch::target::trap::handle_exception(thread)
}

pub fn handle_interrupt(thread: &mut crate::arch::thread::Thread) -> Resume {
    crate::arch::target::trap::handle_interrupt(thread)
}

pub async fn handle_syscall(thread: &mut crate::arch::thread::Thread) -> Resume {
    crate::arch::target::trap::handle_syscall(thread).await
}
