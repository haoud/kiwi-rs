use crate::time::TimerFrequency;

/// The maximum number of CPUs supported by the kernel. This is a compile-time
/// constant that can be adjusted as needed. If you need to support more CPUs,
/// simply increase this value and recompile the kernel. However, this may
/// increase the memory usage of the kernel and reduce performance since Kiwi
/// isn't designed to scale to a large number of CPUs since it targets desktop
/// systems, which typically don't have more than 64 CPUs.
pub const MAX_CPUS: usize = 64;

/// The timer frequency in Hz. This is a compile-time constant that can be
/// adjusted as needed:
/// - A higher frequency will allow for more precise timing and better
///   responsiveness, but will also increase the CPU usage and reduce battery
///   life on laptops. This is ideal for desktop systems or soft real-time
///   systems that require high precision and responsiveness.
/// - A lower frequency will reduce the CPU usage and increase battery life
///   but will also reduce the precision of the timer and the responsiveness
///   of the system. This is ideal for servers and embedded systems that don't
///   require high precision and responsiveness.
///
/// Since Kiwi targets desktop systems, a good default value is 1000 Hz, which
/// provides a good balance between precision and performance.
pub const TIMER_HZ: TimerFrequency = TimerFrequency::new(1000);

/// The maximum number of threads that can be created simultaneously. This is a
/// compile-time constant that can be adjusted as needed. The default value of
/// 32,768 is a reasonable limit for most desktop systems, but increasing this
/// value may be necessary for systems that require a large number of threads
/// at the cost of increasing slightly the memory usage of the kernel.
pub const MAX_THREADS: usize = 32_768;
