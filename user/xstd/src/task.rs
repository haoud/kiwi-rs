/// Terminates the current process with the given exit code.
///
/// # Important
/// This function will invoke a system call to terminate the process and
/// will not return to the caller. This also implies that destructors for
/// any in-scope variables will not be executed, so resources may not be
/// properly released. In general, it is advisable to avoid using this function
/// unless absolutely necessary.
pub fn exit(code: i32) -> ! {
    unsafe {
        core::arch::asm!("ecall",
          in("a7") 1,
          in("a0") code,
          options(noreturn)
        );
    }
}

/// Yields the CPU to the scheduler, allowing other tasks to run. Yielding can
/// increase system responsiveness and improve multitasking performance, and
/// since your task is voluntarily yielding, it may gain priority in the
/// scheduler or be rescheduled more quickly when it becomes runnable again.
pub fn yield_now() {
    // TODO: implement syscall for yielding
}
