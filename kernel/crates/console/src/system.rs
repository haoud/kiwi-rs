//! The information about the system displayed by the console.
use core::time::Duration;

use alloc::{collections::VecDeque, vec::Vec};

use crate::format;

/// The usage of a memory pool, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MemoryUsage {
    pub total: u64,
    pub used: u64,
}

impl MemoryUsage {
    /// Returns the number of bytes that are not used.
    #[must_use]
    pub fn free(&self) -> u64 {
        self.total.saturating_sub(self.used)
    }

    /// Returns the ratio of used memory, in per mille.
    #[must_use]
    pub fn permille(&self) -> u32 {
        format::permille(self.used, self.total)
    }
}

/// The cumulated time spent by a CPU running code and being idle since the
/// boot of the system. The unit is chosen by the system (ticks, nanoseconds,
/// cycles...) since only the ratio between the two values matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CpuTimes {
    pub id: u32,
    pub busy: u64,
    pub idle: u64,
}

/// A provider of information about the system. This is the only way for the
/// console to know what happens in the system, which allows it to run in any
/// environment able to provide this information (a kernel, a userspace
/// program using system calls...).
pub trait System {
    /// Returns the name and the version of the system.
    fn version(&self) -> &str;

    /// Returns the time elapsed since the boot of the system.
    fn uptime(&self) -> Duration;

    /// Returns the usage of the physical memory.
    fn physical_memory(&self) -> MemoryUsage;

    /// Returns the usage of the kernel heap.
    fn heap_memory(&self) -> MemoryUsage;

    /// Appends the cumulated times of each online CPU to `out`.
    fn cpu_times(&self, out: &mut Vec<CpuTimes>);

    /// Returns the number of threads that currently exist in the system.
    fn thread_count(&self) -> usize;
}

/// The load of a CPU during the last sampling period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuLoad {
    pub id: u32,

    /// The ratio of time spent running code, in per mille.
    pub permille: u32,
}

/// The number of samples kept in the history of the statistics.
pub const HISTORY_LENGTH: usize = 128;

/// A snapshot of the state of the system, taken by [`Monitor::refresh`].
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub uptime: Duration,
    pub memory: MemoryUsage,
    pub heap: MemoryUsage,
    pub cpus: Vec<CpuLoad>,
    pub threads: usize,

    /// The average load of the CPUs at the last samples, in per mille, from
    /// the oldest to the newest.
    pub load_history: VecDeque<u64>,

    /// The usage of the physical memory at the last samples, in per mille,
    /// from the oldest to the newest.
    pub memory_history: VecDeque<u64>,
}

impl Stats {
    /// Returns the average load of all CPUs, in per mille.
    #[must_use]
    pub fn total_load(&self) -> u32 {
        let count = u32::try_from(self.cpus.len()).unwrap_or(u32::MAX);
        if count == 0 {
            return 0;
        }
        self.cpus.iter().map(|cpu| cpu.permille).sum::<u32>() / count
    }
}

/// Periodically samples the system to compute statistics that depend on time,
/// such as the load of each CPU during the last period.
#[derive(Debug, Clone, Default)]
pub struct Monitor {
    previous: Vec<CpuTimes>,
    current: Vec<CpuTimes>,
    stats: Stats,
}

impl Monitor {
    /// Creates a new monitor. The CPU loads of the first refresh are averaged
    /// since the boot of the system.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the statistics computed during the last refresh.
    #[must_use]
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Samples the system and updates the statistics.
    pub fn refresh(&mut self, system: &dyn System) {
        core::mem::swap(&mut self.previous, &mut self.current);
        self.current.clear();
        system.cpu_times(&mut self.current);

        self.stats.cpus.clear();
        for now in &self.current {
            let before = self
                .previous
                .iter()
                .find(|cpu| cpu.id == now.id)
                .copied()
                .unwrap_or(CpuTimes {
                    id: now.id,
                    busy: 0,
                    idle: 0,
                });
            let busy = now.busy.saturating_sub(before.busy);
            let idle = now.idle.saturating_sub(before.idle);
            self.stats.cpus.push(CpuLoad {
                id: now.id,
                permille: format::permille(busy, busy + idle),
            });
        }

        self.stats.uptime = system.uptime();
        self.stats.memory = system.physical_memory();
        self.stats.heap = system.heap_memory();
        self.stats.threads = system.thread_count();

        let (load, memory) = (self.stats.total_load(), self.stats.memory.permille());
        push_sample(&mut self.stats.load_history, u64::from(load));
        push_sample(&mut self.stats.memory_history, u64::from(memory));
    }
}

/// Appends a sample to a history, forgetting the oldest sample if the history
/// is full.
fn push_sample(history: &mut VecDeque<u64>, sample: u64) {
    if history.len() == HISTORY_LENGTH {
        history.pop_front();
    }
    history.push_back(sample);
}
