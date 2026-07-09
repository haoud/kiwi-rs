pub use crate::arch::target::thread::{KernelThreadStart, Thread};

/// Executes the given thread. This function will switch to the context of
/// the given thread and start executing it. When the thread is interrupted,
/// its context will be saved and this function will return normally to the
/// caller.
pub fn execute(thread: &mut Thread) {
    crate::arch::target::thread::execute(thread);
}

/// Exits the current thread. This function will terminate the current thread
/// and switch to the next thread in the scheduler.
pub fn exit() -> ! {
    crate::arch::target::thread::exit();
}
