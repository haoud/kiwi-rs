/// The maximum number of tasks that can be created. The kernel will use this
/// constant to allocate memory for the task control blocks and other data
/// during initialization. Diminishing this value will reduce the memory usage
/// of the kernel, but it will also limit the number of tasks that can be run
/// concurrently.
///
/// For a desktop system, the current value of 32 is way too low and should be
/// increased in the future. However, for the current state of the project,
/// this will work well enough.
pub const MAX_TASKS: u16 = 32;

/// The size of the kernel stack. This should be a multiple of the page size,
/// which is 4096 bytes on almost all systems. The kernel stack is used by the
/// kernel to handle syscalls, interrupts, and exceptions.
///
/// Kiwi use a single stack per cpu, so the stack must be large enough to
/// handle all the kernel operations, including nested interrupts and
/// exceptions. A size of 16 KiB should be enough for most use cases, and do
/// not waste too much memory since the stack is only allocated once per CPU.
pub const KERNEL_STACK_SIZE: usize = 4096 * 4;

/// The number of milliseconds that a thread can run continuously before being
/// preempted if it does not yield voluntarily. This value is used to set the
/// timer interrupt frequency for thread scheduling. A smaller value will lead to
/// more frequent context switches, which can improve responsiveness but also
/// increase overhead. A larger value will reduce context switch overhead but
/// may lead to less responsive multitasking.
///
/// The current value of 25 milliseconds is a reasonable compromise for general
/// purpose computing. It provides a good balance between responsiveness and
/// overhead for most workloads. However, this value may need to be adjusted
/// based on the specific requirements of the system and the nature of the tasks
/// being run.
pub const THREAD_MAX_RUN_DURATION: u64 = 25;
