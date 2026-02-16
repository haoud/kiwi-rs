/// The maximum number of CPUs supported by the kernel. This is a compile-time
/// constant that can be adjusted as needed. If you need to support more CPUs,
/// simply increase this value and recompile the kernel. However, this may
/// increase the memory usage of the kernel and reduce performance since Kiwi
/// isn't designed to scale to a large number of CPUs since it targets desktop
/// systems, which typically don't have more than 64 CPUs.
pub const MAX_CPUS: usize = 64;
