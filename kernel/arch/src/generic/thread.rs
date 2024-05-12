pub use crate::target::thread::Thread;
use crate::trap::Trap;

/// Create a new thread with the given instruction pointer and stack pointer.
pub fn create(ip: usize, stack: usize) -> Thread {
    crate::target::thread::create(ip, stack)
}

/// Execute the given thread until a trap occurs and return to the caller.
pub fn execute(thread: &mut Thread) -> Trap {
    crate::target::thread::execute(thread)
}
