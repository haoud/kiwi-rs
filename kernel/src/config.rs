/// The maximum number of tasks that can be created. The kernel will use this
/// constant to allocate memory for the task control blocks and other data
/// during initialization. Diminishing this value will reduce the memory usage
/// of the kernel, but it will also limit the number of tasks that can be run
/// concurrently.
///
/// For a desktop system, the current value of 32 is way too low and should be
/// increased in the future. However, for the current state of the project, this
/// will work well enough.
pub const MAX_TASKS: u16 = 32;

/// The size of the kernel stack. This should be a multiple of the page size, which
/// is 4096 bytes on almost all systems. The kernel stack is used by the kernel to
/// handle syscalls, interrupts, and exceptions.
///
/// Kiwi use a single stack per cpu, so the stack must be large enough to handle
/// all the kernel operations, including nested interrupts and exceptions.
pub const KERNEL_STACK_SIZE: usize = 4096;
