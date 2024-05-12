#![no_std]

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
/// exceptions.
pub const KERNEL_STACK_SIZE: usize = 4096;

/// The number of milliseconds that a thread can run before being preempted
/// by the scheduler if it has not yielded the CPU. This value is used to
/// prevent a single thread from monopolizing the CPU.
///
/// The value of 25 milliseconds is chosen because it is a good balance
/// between responsiveness and performance. A lower value would make the
/// system more responsive, but it would also increase the number of context
/// switches, which would reduce the overall throughput of the system and
/// increase the overhead of the scheduler. A higher value would reduce the
/// number of context switches and reduce the overhead of the scheduler, but
/// it would also make the system less responsive.
///
/// Depending on the use case, this value can be adjusted to better fit the
/// requirements of the system.
pub const THREAD_QUANTUM: u64 = 25;
