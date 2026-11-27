use alloc::vec::Vec;

use console::system::{CpuTimes, MemoryUsage, System};

use crate::{
    arch::addr::PAGE_SIZE,
    mm, scheduler,
    time::{duration::Duration, jiffies},
};

/// The information about the kernel displayed by the console.
pub struct KernelSystem;

impl System for KernelSystem {
    /// Returns the name of the kernel and the version of its crate.
    fn version(&self) -> &'static str {
        concat!("Kiwi ", env!("CARGO_PKG_VERSION"))
    }

    /// Returns the time elapsed since boot, with the precision of the jiffies.
    fn uptime(&self) -> core::time::Duration {
        core::time::Duration::from_nanos(Duration::from(jiffies::jiffies()).as_nanos())
    }

    /// Returns the usage of the physical memory, computed from the metadata
    /// of each physical page. The reserved and poisoned pages are not counted
    /// since they cannot be used.
    ///
    /// This is slow since every page metadata must be locked, but it is only
    /// called when the console refreshes its information.
    fn physical_memory(&self) -> MemoryUsage {
        let stats = mm::page::metadata().statistics();
        let usable = stats.total.0 - stats.reserved.0 - stats.poisoned.0;
        let bytes = |pages: usize| (pages * PAGE_SIZE) as u64;
        MemoryUsage {
            total: bytes(usable),
            used: bytes(usable.saturating_sub(stats.free.0)),
        }
    }

    fn heap_memory(&self) -> MemoryUsage {
        let stats = mm::heap::statistics();
        MemoryUsage {
            total: stats.size as u64,
            used: stats.used as u64,
        }
    }

    /// Returns the number of busy and idle timer ticks of each CPU, accounted
    /// by the scheduler.
    fn cpu_times(&self, out: &mut Vec<CpuTimes>) {
        out.extend(scheduler::load::cpu_times().map(|times| CpuTimes {
            id: u32::from(times.cpu),
            busy: times.busy,
            idle: times.idle,
        }));
    }

    fn thread_count(&self) -> usize {
        scheduler::task_count()
    }
}
