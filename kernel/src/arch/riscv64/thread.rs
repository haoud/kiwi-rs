use super::{mmu, trap};
use crate::arch::trap::Trap;
use alloc::boxed::Box;
use riscv::register::scause::{self, Exception};

core::arch::global_asm!(include_str!("asm/thread.asm"));

unsafe extern "C" {
    fn thread_execute(context: &mut trap::Context);
}

/// A thread is a sequence of instructions that can be executed independently
/// of other code. On RISC-V, a thread is represented by a `Context` that
/// contains a copy of all the registers and a `Table` that contains the page
/// table of the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    context: Box<trap::Context>,
    table: Box<mmu::Table>,
}

impl Thread {
    /// Create a new thread with an empty page table.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: Box::new(trap::Context::new()),
            table: Box::new(mmu::Table::empty()),
        }
    }

    /// Return a mutable reference to the context of the thread.
    #[must_use]
    pub fn context_mut(&mut self) -> &mut trap::Context {
        &mut self.context
    }

    /// Return a reference to the context of the thread.
    #[must_use]
    pub fn context(&self) -> &trap::Context {
        &self.context
    }

    /// Return a mutable reference to the page table of the thread.
    #[must_use]
    pub fn table_mut(&mut self) -> &mut mmu::Table {
        &mut self.table
    }

    /// Return a reference to the page table of the thread.
    #[must_use]
    pub fn table(&self) -> &mmu::Table {
        &self.table
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        log::trace!("Dropping thread");
    }
}

/// Create a new thread with the given instruction pointer and stack pointer.
/// This will create a thread with a default context and an empty user page
/// table (but still containing the kernel mappings).
#[must_use]
pub fn create(ip: usize, stack: usize) -> Thread {
    let mut thread = Thread::new();
    thread.table.setup_from_kernel_space();
    thread.context.set_sp(stack);
    thread.context.set_ip(ip);
    thread
}

/// Save state of the current thread that was not saved by the trap handler
/// for efficient trap handling. The trap handler will only save the minimal
/// state required to run the trap handler without conflicting with the
/// thread state. This often means that only the CPU registers (used by the
/// kernel) are saved prior to handling the trap. State that is not altered
/// by the kernel, such as the FPU state, can be saved before a context switch
/// to avoid the overhead of saving and restoring the state on every trap.
pub fn save(_thread: &mut Thread) {
    // TODO: Save FPU state
}

/// Execute the current thread. This function will switch to the page table
/// of the thread and execute the thread. When a trap will occur, the trap
/// handler will be called and the thread will be paused. The trap handler
/// will invoke some incantations and will return to the caller of this
/// function.
pub fn execute(thread: &mut Thread) -> Trap {
    // TODO: Restore FPU state
    unsafe {
        thread.table().set_current();
        thread_execute(&mut thread.context);
    }

    match riscv::register::scause::read().cause() {
        scause::Trap::Exception(Exception::UserEnvCall) => Trap::Syscall,
        scause::Trap::Exception(_) => Trap::Exception,
        scause::Trap::Interrupt(_) => Trap::Interrupt,
    }
}

/// Get the syscall identifier from the given thread. On RISC-V, the
/// syscall identifier is stored in the a7 register (x17).
#[must_use]
pub fn get_syscall_id(thread: &Thread) -> usize {
    thread.context.get_register(17)
}

/// Get the raw syscall arguments from the given thread. On RISC-V, the
/// syscall arguments are stored in the a0-a5 registers (x10-x15).
#[must_use]
pub fn get_syscall_args(thread: &Thread) -> [usize; 6] {
    [
        thread.context.get_register(10),
        thread.context.get_register(11),
        thread.context.get_register(12),
        thread.context.get_register(13),
        thread.context.get_register(14),
        thread.context.get_register(15),
    ]
}
