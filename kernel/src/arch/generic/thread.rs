pub use crate::arch::target::thread::Thread;
use crate::arch::trap::Trap;

/// Create a new thread with the given instruction pointer and stack pointer.
#[must_use]
pub fn create(ip: usize, stack: usize) -> Thread {
    crate::arch::target::thread::create(ip, stack)
}

/// Execute the given thread until a trap occurs and return to the caller.
#[must_use]
pub fn execute(thread: &mut Thread) -> Trap {
    crate::arch::target::thread::execute(thread)
}

/// Get the syscall identifier from the given thread.
#[must_use]
pub fn get_syscall_id(thread: &Thread) -> usize {
    crate::arch::target::thread::get_syscall_id(thread)
}

/// Get the raw syscall arguments from the given thread.
#[must_use]
pub fn get_syscall_args(thread: &Thread) -> [usize; 6] {
    crate::arch::target::thread::get_syscall_args(thread)
}

/// Set the return value of the syscall for the given thread.
pub fn set_syscall_return(thread: &mut Thread, value: isize) {
    crate::arch::target::thread::set_syscall_return(thread, value);
}
