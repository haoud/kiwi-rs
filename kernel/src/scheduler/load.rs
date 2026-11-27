//! Accounting of the time spent by each CPU running code and being idle.
//!
//! The load is sampled at each timer tick: the tick is accounted as idle if
//! the CPU was waiting for an interrupt in [`idle_wait`] when the tick
//! occurred, and as busy otherwise. This is cheap and, with a 1 kHz timer,
//! precise enough to display the load of the CPUs.
use core::{
    cell::Cell,
    sync::atomic::{AtomicU64, Ordering},
};

use macros::per_cpu;

use crate::{arch, config::MAX_CPUS};

/// The number of ticks a CPU spent busy and idle since it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuTimes {
    /// The identifier of the CPU.
    pub cpu: u8,

    /// The number of timer ticks during which the CPU was running code.
    pub busy: u64,

    /// The number of timer ticks during which the CPU was idle.
    pub idle: u64,
}

/// The tick counters of a CPU. They are only written by their CPU, but can be
/// read from any CPU.
struct Counters {
    busy: AtomicU64,
    idle: AtomicU64,
}

/// The tick counters of all the CPUs, indexed by CPU identifier.
static COUNTERS: [Counters; MAX_CPUS] = [const {
    Counters {
        busy: AtomicU64::new(0),
        idle: AtomicU64::new(0),
    }
}; MAX_CPUS];

/// Whether the current CPU is waiting for an interrupt in [`idle_wait`].
#[per_cpu]
static IDLE: Cell<bool> = Cell::new(false);

/// Puts the current CPU in a low-power state until the next interrupt, and
/// accounts the time spent waiting as idle time. This must be used instead of
/// [`arch::irq::wait`] when the CPU has nothing to do.
pub fn idle_wait() {
    IDLE.local().set(true);
    arch::irq::wait();
    IDLE.local().set(false);
}

/// Accounts a timer tick to the current CPU, as idle or busy depending on
/// whether the CPU was waiting in [`idle_wait`]. This must be called by the
/// timer interrupt handler of each CPU.
pub fn account_tick() {
    let Some(counters) = COUNTERS.get(usize::from(arch::smp::cpu_identifier())) else {
        return;
    };
    if IDLE.local().get() {
        counters.idle.fetch_add(1, Ordering::Relaxed);
    } else {
        counters.busy.fetch_add(1, Ordering::Relaxed);
    }
}

/// Returns the times of each CPU that has received at least one timer tick,
/// sorted by CPU identifier.
pub fn cpu_times() -> impl Iterator<Item = CpuTimes> {
    COUNTERS.iter().enumerate().filter_map(|(cpu, counters)| {
        let busy = counters.busy.load(Ordering::Relaxed);
        let idle = counters.idle.load(Ordering::Relaxed);
        (busy + idle > 0).then(|| CpuTimes {
            cpu: u8::try_from(cpu).unwrap_or(u8::MAX),
            busy,
            idle,
        })
    })
}
