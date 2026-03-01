use core::{cell::UnsafeCell, ops::Range};

use bitflags::bitflags;
use macros::init;

use crate::{
    arch::{
        self,
        addr::{AllMemory, Physical},
    },
    library::lock::spin::Spinlock,
    mm::page,
};

bitflags! {
    /// Flags representing the state and usage of a physical page.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u16 {
        /// The page is free and can be allocated for general-purpose use.
        const FREE = 1 << 0;

        /// The page is used by the kernel.
        const KERNEL = 1 << 1;

        /// The page is reserved by the hardware.
        const RESERVED = 1 << 2;

        /// The page is poisoned and cannot be used for any purpose. This is
        /// used to mark pages that are damaged or do not belong to any other
        /// page kind.
        const POISONED = 1 << 3;

        /// The page is the head of a contiguous block of pages used by the
        /// buddy allocator. The contiguous block of pages may or may not be
        /// free depending on the state of the buddy allocator, but the head
        /// page is always marked with this flag to ensure that the buddy
        /// allocator can find the metadata for the block of pages when it
        /// needs to without risking corruption if a wrong page is accessed.
        const BUDDY = 1 << 4;
    }
}

/// Represents a count of physical pages in the system.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Count(pub usize);

impl Count {
    /// Creates a page count from a range of physical addresses, rounding
    /// up to the nearest page boundary.
    #[must_use]
    pub const fn from_range(range: Range<Physical<AllMemory>>) -> Self {
        Self::from_bytes(range.end.as_usize() - range.start.as_usize())
    }

    /// Creates a page count from a byte count, rounding up to the nearest
    /// page boundary.
    #[must_use]
    pub const fn from_bytes(bytes: usize) -> Self {
        Self(bytes.div_ceil(arch::addr::PAGE_SIZE))
    }

    /// Converts the page count to a byte count
    #[must_use]
    pub const fn to_bytes(&self) -> usize {
        self.0 * arch::addr::PAGE_SIZE
    }

    /// Converts the page count to a kibibyte count, rounding down to the
    /// nearest kibibyte.
    #[must_use]
    pub const fn to_kibibytes(&self) -> usize {
        self.to_bytes() / 1024
    }

    /// Converts the page count to a mebibyte count, rounding down to the
    /// nearest mebibyte.
    #[must_use]
    pub const fn to_mebibytes(&self) -> usize {
        self.to_kibibytes() / 1024
    }

    /// Converts the page count to a gibibyte count, rounding down to the
    /// nearest gibibyte.
    #[must_use]
    pub const fn to_gibibytes(&self) -> usize {
        self.to_mebibytes() / 1024
    }
}

impl core::fmt::Display for Count {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::ops::Add for Count {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Add<usize> for Count {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::AddAssign for Count {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl core::ops::AddAssign<usize> for Count {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl core::ops::Sub for Count {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl core::ops::Sub<usize> for Count {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl core::ops::SubAssign for Count {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl core::ops::SubAssign<usize> for Count {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl From<usize> for Count {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Statistics about the physical pages in the system.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct PagesStatistics {
    /// The total number of physical pages in the system.
    pub total: page::Count,

    /// The number of free physical pages in the system.
    pub free: page::Count,

    /// The number of physical pages used by the kernel.
    pub kernel: page::Count,

    /// The number of reserved physical pages in the system.
    pub reserved: page::Count,

    /// The number of poisoned physical pages in the system.
    pub poisoned: page::Count,
}

impl PagesStatistics {
    /// Prints the statistics in a human-readable format to the kernel console.
    pub fn print_debug_output(&self) {
        log::debug!(
            "Physical memory: {} KiB total, {} KiB free, {} KiB used by kernel, \
        {} KiB reserved, {} KiB poisoned",
            self.total.to_kibibytes(),
            self.free.to_kibibytes(),
            self.kernel.to_kibibytes(),
            self.reserved.to_kibibytes(),
            self.poisoned.to_kibibytes(),
        );
    }
}

/// Metadata of a physical page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// The number of entities currently using the page. This is used to track
    /// how many entities are currently using the page, and to determine when a
    /// page can be safely freed or reused.
    usage: u16,

    /// A set of flags representing the state and usage of the page.
    pub flags: page::Flags,
}

impl Metadata {
    /// Creates a new page metadata with the given flags and a usage count of zero.
    #[must_use]
    pub const fn new(flags: page::Flags) -> Self {
        Self { flags, usage: 0 }
    }

    /// Increases the usage count of the page by one. This is used to track
    /// how many entities are currently using the page.
    ///
    /// # Counter overflow
    /// If the usage count reaches `u16::MAX`, it will not be increased further
    /// to avoid overflow. In this case, the page will be considered pinned in
    /// memory to prevent use-after-free bugs, and a warning will be logged to
    /// indicate that the usage count has overflowed and it will not possible
    /// to modify the usage count of the page anymore. This is a safety measure
    /// to prevent undefined behavior in case of usage count overflow at the
    /// cost of a memory leak.
    pub fn retain(&mut self) {
        self.usage = self.usage.saturating_add(1);
        if self.usage == u16::MAX {
            log::warn!(
                "Page usage count overflowed: \
                page pinned in memory to avoid use-after-free bugs"
            );
        }
    }

    /// Decreases the usage count of the page by one, and returns `true` if
    /// the usage count has reached zero, indicating that the page is no longer
    /// in use.
    pub fn dispose(&mut self) -> bool {
        if self.usage == 0 {
            log::error!("Disposing a page with zero usage count");
        } else if self.usage != u16::MAX {
            self.usage -= 1;
        }
        self.usage == 0
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new(page::Flags::POISONED)
    }
}

/// The metadata table for physical pages. This is a global table that contains
/// metadata for each physical page in the system. To reduce the memory overhead
/// of the metadata table, we only store page metadata until the end of the last
/// regular address in the system, which is the highest address that can be used
/// for general-purpose memory allocation. There may exist valid physical pages
/// beyond this address, but they are reserved for special purposes (most likely
/// for memory-mapped devices) and they will not be usable for general-purpose
/// memory allocation, so we can safely ignore them.
#[derive(Debug)]
pub struct MetadataTable {
    table: UnsafeCell<&'static [Spinlock<Metadata>]>,
}

/// SAFETY: The metadata table slice is initialized during the kernel setup
/// phase and is not modified after that point. Therefore, considering that
/// I decided that the initialization code take responsibility for ensuring
/// that no undefined behavior occurs during the initialization phase, it is
/// safe to share the metadata table across threads without synchronization
/// after the setup phase is complete.
unsafe impl Sync for MetadataTable {}

impl MetadataTable {
    /// Creates an uninitialized metadata table.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            table: UnsafeCell::new(&[]),
        }
    }

    /// Initializes the metadata table by reclaiming the boot memory map and
    /// populating the table with metadata for each physical page.
    ///
    /// # Safety
    /// This function should only be called once during the kernel
    /// initialization.
    ///
    /// # Panics
    /// Panics if the boot memory map was already reclaimed.
    #[init]
    pub unsafe fn setup(&'static self) {
        let producer = || Spinlock::new(Metadata::default());
        let count = arch::boot::last_regular_address().frame_idx();

        // Allocate a slice of metadata entries for all physical pages in the
        // system then reclaim the boot memory map to populate the metadata for
        // each page in the system.
        let table = arch::boot::allocate_slice_from_fn(count, producer);
        let mmap = arch::boot::reclaim_memory();

        // Iterate over the memory map and update the metadata for each
        // physical page based on the memory kind of the region it belongs to.
        // Pages that do not belong to any region in the memory map will be
        // marked as poisoned and will not be usable for any purpose.
        for entry in &mmap.regions {
            for mut frame in table
                .iter_mut()
                .take(entry.end.frame_idx())
                .skip(entry.start.frame_idx())
                .map(|lock| lock.lock())
            {
                match entry.kind {
                    arch::mem::MemoryKind::Free => {
                        frame.flags.remove(page::Flags::POISONED);
                        frame.flags.insert(page::Flags::FREE);
                    }
                    arch::mem::MemoryKind::Kernel => {
                        frame.flags.remove(page::Flags::POISONED);
                        frame.flags.insert(page::Flags::KERNEL);
                        frame.usage = 1;
                    }
                    arch::mem::MemoryKind::Reserved => {
                        frame.flags.remove(page::Flags::POISONED);
                        frame.flags.insert(page::Flags::RESERVED);
                    }
                    arch::mem::MemoryKind::Poisoned => {}
                }
            }
        }

        self.table.replace(table);
    }

    /// Returns a reference to the metadata table.
    #[must_use]
    pub fn table(&self) -> &'static [Spinlock<Metadata>] {
        // SAFETY: This is safe since the table slice is only modified
        // during the setup phase. After this phase, the table slice
        // is immutable and can be safely shared across threads without
        // synchronization.
        unsafe { &*self.table.get() }
    }

    /// Collects and returns statistics about the physical pages in the
    /// system. This function is slow and should not be called frequently
    /// since it needs to lock every page's metadata to collect accurate
    /// statistics.
    #[must_use]
    pub fn statistics(&self) -> PagesStatistics {
        let mut stats = PagesStatistics::default();
        for frame in self.table().iter().map(|lock| lock.lock()) {
            stats.total += 1;
            if frame.flags.contains(page::Flags::FREE) {
                stats.free += 1;
            }
            if frame.flags.contains(page::Flags::KERNEL) {
                stats.kernel += 1;
            }
            if frame.flags.contains(page::Flags::RESERVED) {
                stats.reserved += 1;
            }
            if frame.flags.contains(page::Flags::POISONED) {
                stats.poisoned += 1;
            }
        }
        stats
    }
}

/// The global metadata table for physical pages.
static PAGE_METADATA: MetadataTable = MetadataTable::empty();

/// Initializes the page metadata table by reclaiming the boot memory map and
/// populating the table with metadata for each physical page.
///
/// # Safety
/// This function should only be called once during the kernel initialization.
#[init]
pub unsafe fn setup() {
    PAGE_METADATA.setup();
    PAGE_METADATA.statistics().print_debug_output();
}

/// Returns a reference to the global metadata table for physical pages.
#[must_use]
pub fn table() -> &'static [Spinlock<Metadata>] {
    PAGE_METADATA.table()
}
