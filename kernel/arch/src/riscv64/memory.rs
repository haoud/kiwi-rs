use crate::{generic::memory::UsableMemory, memory::Region};
use heapless::Vec;

impl UsableMemory {
    /// Create a new `UsableMemory` structure from the device tree given
    /// as argument.
    #[inline]
    #[must_use]
    pub fn new(device_tree: &fdt::Fdt) -> Self {
        let mut regions = Vec::<Region, 32>::new();

        // Iterate over all the memory regions in the device tree and add
        // them to the usable memory regions
        for region in device_tree.memory().regions() {
            let mut start = region.starting_address as usize;
            let mut length = region.size.unwrap_or(0);

            ::log::info!(
                "Available memory region: {:#010x} - {:#010x}",
                start,
                start + length
            );

            // The region 0x80000000 to 0x80200000 is reserved for the firmware
            // The region 0x80200000 to 0x80300000 is reserved for the kernel
            // TODO: Dynamically allocate the firmware and kernel region
            if start < 0x80300000 {
                length = length.saturating_sub(0x80300000 - start);
                start = 0x80300000;
            }

            regions
                .push(Region { start, length })
                .expect("Failed to push region");
        }

        Self { regions }
    }
}
