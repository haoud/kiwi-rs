use super::mmu;
use crate::arch::{generic::memory::UsableMemory, memory::Region};
use heapless::Vec;

extern "C" {
    static __reclaimable_start: [u8; 0];
    static __reclaimable_end: [u8; 0];
    static __start: [u8; 0];
    static __end: [u8; 0];
}

impl UsableMemory {
    /// Create a new `UsableMemory` structure from the device tree given
    /// as argument.
    ///
    /// # Panics
    /// This function will panic if there is no information about the memory
    /// regions in the device tree, or if there are too many memory regions
    /// in the device tree that we cannot handle.
    #[inline]
    #[must_use]
    pub fn new(device_tree: &fdt::Fdt) -> Self {
        // Compute the kernel start and end addresses in physical memory, so
        // that we can skip the kernel memory region when adding the memory
        // regions to the usable memory to avoid overwriting ourselves :(
        let kernel_physical_start = unsafe {
            usize::from(mmu::translate_kernel_ptr(core::ptr::addr_of!(__start)))
        };
        let kernel_physical_end = unsafe {
            usize::from(mmu::translate_kernel_ptr(core::ptr::addr_of!(__end)))
        };
        let kernel_reclaimable_start = unsafe {
            usize::from(mmu::translate_kernel_ptr(core::ptr::addr_of!(
                __reclaimable_start
            )))
        };
        let kernel_reclaimable_end = unsafe {
            usize::from(mmu::translate_kernel_ptr(core::ptr::addr_of!(
                __reclaimable_end
            )))
        };

        let kernel_memory = kernel_physical_end - kernel_physical_start;
        let firmware_memory = 0x0020_0000;
        let total_memory = device_tree
            .memory()
            .regions()
            .map(|r| r.size.unwrap_or(0))
            .sum::<usize>();

        let ram_start = device_tree
            .memory()
            .regions()
            .map(|region| region.starting_address as usize)
            .min()
            .unwrap();

        let ram_end = device_tree
            .memory()
            .regions()
            .map(|region| {
                let start = region.starting_address as usize;
                start + region.size.unwrap_or(0)
            })
            .max()
            .unwrap();

        log::info!("Total memory: {} kiB", total_memory / 1024);
        log::info!("Firmware memory: {} KiB", firmware_memory / 1024);
        log::info!("Kernel memory: {} kiB", kernel_memory / 1024);
        log::debug!(
            "Reclaimable memory: {} KiB",
            (kernel_reclaimable_end - kernel_reclaimable_start) / 1024
        );
        log::debug!("RAM start: 0x{:016x}", ram_start);
        log::debug!("RAM end: 0x{:016x}", ram_end);

        // Iterate over all the memory regions in the device tree and add
        // them to the usable memory regions
        let mut regions = Vec::<Region, 32>::new();
        for region in device_tree.memory().regions() {
            let mut start = region.starting_address as usize;
            let mut length = region.size.unwrap_or(0);

            // The region 0x80000000 to 0x80200000 is reserved for the firmware
            // The region kernel_start (0x80200000) to kernel_end is reserved
            // for the kernel static code and data
            // FIXME: Does assuming that the RAM cannot start before 0x80000000
            // is true for all riscv64 platforms ?
            if start < kernel_physical_end {
                length -= kernel_physical_end - 0x8000_0000;
                start = kernel_physical_end;
            }

            regions
                .push(Region { start, length })
                .expect("Failed to push region");

            ::log::debug!(
                "Available memory region: {:#010x} - {:#010x}",
                start,
                start + length
            );
        }

        Self {
            regions,
            firmware_memory,
            kernel_memory,
            total_memory,
            ram_start,
            ram_end,
        }
    }
}
