use crate::arch::mmu::{self, Physical};
use heapless::Vec;

/// A structure representing the usable memory regions of the system. It is
/// used to allocate memory for objects during the initialization of the kernel
/// before giving the remaning memory to the pager process.
#[derive(Debug, Clone)]
pub struct UsableMemory {
    /// The list of memory regions that can be used to allocate memory.
    pub regions: Vec<Region, 32>,

    /// The amount of memory reserved for the firmware.
    pub firmware_memory: usize,

    /// The amount of memory reserved for the kernel.
    pub kernel_memory: usize,

    /// The total amount of memory available.
    pub total_memory: usize,
}

impl UsableMemory {
    /// Create a new usable memory structure with the given memory regions and
    /// memory informations.
    #[must_use]
    fn setup(
        regions: Vec<Region, 32>,
        firmware_memory: usize,
        kernel_memory: usize,
        total_memory: usize,
    ) -> Self {
        Self {
            regions,
            firmware_memory,
            kernel_memory,
            total_memory,
        }
    }

    /// Allocate an object using the available memory regions. It will update
    /// the region list to reflect the allocation and will return a physical
    /// address that can be used to store the object.
    #[must_use]
    pub fn allocate_memory<T>(
        &mut self,
        length: usize,
        align: usize,
    ) -> Option<Physical> {
        // Verify that the alignment given is at least the minimum alignment
        // required for the type T.
        if align < core::mem::align_of::<T>() {
            ::log::error!(
                "Object {} requires an alignment of at least {}, but {} was given",
                core::any::type_name::<T>(),
                core::mem::align_of::<T>(),
                align
            );
            return None;
        }

        // If the alignment is not a power of two, the allocation is invalid
        // and we must return None.
        if !align.is_power_of_two() {
            ::log::error!("Invalid alignment for allocation of type T");
            return None;
        }

        // Find a region that can fit the allocation with the given alignment
        // and update the region list to reflect the allocation.
        let region = self
            .regions
            .iter_mut()
            .find(|region| region.length >= length * 2)
            .map(|region| {
                // Align the start of the region
                let align = (align - (region.start % align)) % align;
                let start = region.start + align;
                region.start += length + align;
                region.length -= length + align;

                self.kernel_memory += length + align;
                Region { start, length }
            })?;

        // Return the allocated pointer
        Some(Physical(region.start))
    }

    /// Allocate a page of memory using the available memory regions. It will
    /// update the region list to reflect the allocation and will return a
    /// pointer to the allocated **physical** page. The page is guaranteed to
    /// be zeroed.
    #[must_use]
    pub fn allocate_zeroed_page(&mut self) -> Option<Physical> {
        let page = self.allocate_page()?;
        unsafe {
            core::ptr::write_bytes(
                crate::arch::mmu::translate_physical(page).unwrap().0
                    as *mut u8,
                0,
                4096,
            );
        }
        Some(page)
    }

    /// Allocate a page of memory using the available memory regions. It will
    /// update the region list to reflect the allocation and will return a
    /// pointer to the allocated **physical** page.
    #[must_use]
    pub fn allocate_page(&mut self) -> Option<Physical> {
        self.allocate_memory::<[u8; 4096]>(4096, 4096)
    }

    /// Find the last usable address in the memory regions. This address is
    /// guaranteed to be page aligned. If no regions are available, the
    /// function will return 0.
    #[must_use]
    pub fn last_address(&self) -> mmu::Physical {
        // Find the region with the highest end address
        self.regions
            .iter()
            .max_by_key(|region| region.start + region.length)
            .map(|region| mmu::Physical(region.start + region.length))
            .map(|addr| addr.page_align_down())
            .unwrap_or(mmu::Physical(0))
    }

    /// Convert the usable memory into a list of free memory regions.
    #[must_use]
    pub fn into_free_regions(self) -> Vec<Region, 32> {
        self.regions
    }

    /// Return the size of the usable memory regions in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.regions.iter().map(|region| region.length).sum()
    }
}

/// A structure representing a memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    pub start: usize,
    pub length: usize,
}

impl Region {
    /// Return the end address of the region
    #[must_use]
    pub fn end(&self) -> mmu::Physical {
        mmu::Physical(self.start + self.length)
    }
}
