pub use crate::arch::target::thread::Thread;
use crate::arch::trap::Trap;

/// Create a new thread with the given instruction pointer and stack pointer.
pub fn create(ip: usize, stack: usize) -> Thread {
    crate::arch::target::thread::create(ip, stack)
}

/// Execute the given thread until a trap occurs and return to the caller.
pub fn execute(thread: &mut Thread) -> Trap {
    crate::arch::target::thread::execute(thread)
}
