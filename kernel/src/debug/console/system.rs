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
    fn version(&self) -> &'static str {
        concat!("Kiwi [Version ", env!("CARGO_PKG_VERSION"), "]")
    }

    fn uptime(&self) -> core::time::Duration {
        core::time::Duration::from_nanos(Duration::from(jiffies::jiffies()).as_nanos())
    }

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
