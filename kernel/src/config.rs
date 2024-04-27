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
