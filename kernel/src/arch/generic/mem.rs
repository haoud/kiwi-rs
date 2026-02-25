use arrayvec::ArrayVec;

use crate::arch::addr::{AllMemory, Physical};

/// A memory map describes the layout of physical memory.
#[derive(Debug, Clone)]
pub struct MemoryMap {
    /// A list of memory regions that describe the layout of physical memory.
    pub regions: ArrayVec<Region, { Self::MAX_REGIONS }>,
}

impl MemoryMap {
    /// The maximum number of memory regions that can be stored in the memory
    /// map. If the memory map contains more regions than this, the kernel will
    /// panic. The default value should be large enough to accommodate most
    /// memory maps without wasting too much memory.
    const MAX_REGIONS: usize = 64;

    /// Creates a new empty memory map.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            regions: ArrayVec::new(),
        }
    }
}

/// A memory region describes a contiguous range of physical memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub start: Physical<AllMemory>,
    pub end: Physical<AllMemory>,
    pub kind: MemoryKind,
}

impl Region {
    /// Returns the length of the region in bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end.as_usize() - self.start.as_usize()
    }

    /// Returns `true` if the region is empty, i.e. its length is zero.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The kind of memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// The region does not contain any meaningful data and can be reclaimed
    /// and used for general-purpose allocations.
    Free,

    /// The region contains the kernel and its data structures. The region is
    /// still regular memory, but cannot be used for allocations.
    Kernel,

    /// The region is reserved for hardware devices and cannot be used for
    /// general-purpose allocations.
    Reserved,

    /// The region is poisoned and cannot be used for any purpose. This is
    /// used to mark regions that are damaged or do not belong to any
    /// other memory kind.
    Poisoned,
}
