use crate::target::mmu::Table;
pub use crate::target::mmu::{PAGE_SHIFT, PAGE_SIZE};
use bitflags::bitflags;
use core::ops::{Add, Sub};

/// A physical address. This is used to provide a type-safe way to represent
/// physical addresses, and its implementation varies depending on the architecture.
///
/// A physical address represents a location in the physical memory of the system.
/// Each physical address is unique, but cannot be used to access the memory and must
/// be mapped to a virtual address before being used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Physical(pub(crate) usize);

impl Physical {
    /// Return the index of the frame that contains the physical address. This is
    /// useful to find the frame that contains the physical address, which is used
    /// to map the physical address to a virtual address.
    #[must_use]
    pub const fn frame_idx(self) -> usize {
        self.0 / PAGE_SIZE
    }

    /// Convert a physical address to a `usize`.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Align down the physical address to the given alignment. The alignment must be
    /// a power of two, otherwise the result will be incorrect. If the physical address
    /// is already aligned to the given alignment, the address will not be changed.
    #[must_use]
    pub const fn align_down(self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self(self.0 & !(align - 1))
    }

    /// Align up the physical address to the given alignment. The alignment must be
    /// a power of two, otherwise the result will be incorrect. If the physical address
    /// is already aligned to the given alignment, the address will not be changed.
    #[must_use]
    pub const fn align_up(self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Verify if the physical address is aligned to the given alignment. The alignment
    /// must be a power of two, otherwise the result will be incorrect.
    #[must_use]
    pub const fn is_aligned(self, align: usize) -> bool {
        debug_assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }
}

impl From<Physical> for usize {
    fn from(physical: Physical) -> Self {
        physical.0
    }
}

impl Add<usize> for Physical {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl Sub<usize> for Physical {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

/// A virtual address. This is used to provide a type-safe way to represent
/// virtual addresses, and its implementation varies depending on the architecture.
///
/// A virtual address represents a location in the virtual memory of the system.
/// Different virtual addresses can point to the same physical address, and the
/// translation between virtual and physical addresses is done by the Memory Management
/// Unit (MMU) of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Virtual(pub(crate) usize);

impl Virtual {
    /// Convert a virtual address to a `usize`.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Align down the virtual address to the given alignment. The alignment must be
    /// a power of two, otherwise the result will be incorrect. If the virtual address
    /// is already aligned to the given alignment, the address will not be changed.
    #[must_use]
    pub const fn align_down(self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self(self.0 & !(align - 1))
    }

    /// Align up the virtual address to the given alignment. The alignment must be
    /// a power of two, otherwise the result will be incorrect. If the virtual address
    /// is already aligned to the given alignment, the address will not be changed.
    #[must_use]
    pub const fn align_up(self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Verify if the virtual address is aligned to the given alignment. The alignment
    /// must be a power of two, otherwise the result will be incorrect.
    #[must_use]
    pub const fn is_aligned(self, align: usize) -> bool {
        debug_assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }

    /// Create a new virtual address from a raw pointer.
    #[must_use]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self(ptr as usize)
    }

    /// Convert the virtual address to a mutable raw pointer.
    #[must_use]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Convert the virtual address to a raw pointer.
    #[must_use]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }
}

impl From<Virtual> for usize {
    fn from(virt: Virtual) -> Self {
        virt.0
    }
}

impl Add<usize> for Virtual {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl Sub<usize> for Virtual {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

bitflags! {
    /// A set of rights that can be granted to a memory region. These rights are used
    /// to control the access to the memory region, and are enforced by the Memory
    /// Management Unit (MMU) of the system. For some architectures, some of these
    /// rights may not be supported (e.g the `EXECUTE` right on some x86 systems) or
    /// may be implicit (e.g the `READ` is always granted on x86 systems).
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Rights: u32 {
        const USER = 1 << 0;
        const READ = 1 << 1;
        const WRITE = 1 << 2;
        const EXECUTE = 1 << 3;

        const RX = Self::READ.bits() | Self::EXECUTE.bits();
        const RW = Self::READ.bits() | Self::WRITE.bits();
        const RWX = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }

    /// A set of flags that can be used to control the behavior of a memory region.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u32 {
        /// The memory region is global and should not be flushed from the TLB when
        /// changing the address space. This must be only used if a page is shared
        /// between all address spaces. Otherwise, it may lead to security issues or
        /// strange bugs that will be very, very hard to debug.
        const GLOBAL = 1 << 0;

        /// Use a page size of 2 MiB instead of the default 4 KiB. This can be used to
        /// reduce the number of entries in the page table and improve the performance
        /// of the system and reduce the memory usage. However, the given physical
        /// address must be aligned to 2 MiB.
        const HUGE_2MB = 1 << 1;

        /// Use a page size of 1 GiB instead of the default 4 KiB. This can be used to
        /// reduce the number of entries in the page table and improve the performance
        /// of the system and reduce the memory usage. However, the given physical
        /// address must be aligned to 1 GiB.
        const HUGE_1GB = 1 << 2;
    }
}

/// An error that can happen when trying to map a physical address to a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// A invalid combination of flags was given.
    InvalidFlagsCombination,

    /// The frame is not aligned to the required size. The frame must be aligned to
    /// either 4 KiB, 2 MiB or 1 GiB, depending on the flags given.
    FrameNotAligned,

    /// The given frame was already mapped to another virtual address. The caller
    /// should unmap the frame from the other virtual address before trying to map
    /// it again.
    AlreadyMapped,

    /// The given frame could not be mapped because it would require an intermediate
    /// table to be created, and the given frame cannot be consumed to create it. This
    /// is an variant of `FrameConsumed` that is mostly returned when the frame size is
    /// greater than table size.
    NeedIntermediateTable,

    /// The given frame was consumed to create an intermediate table. The caller
    /// should retry the operation with another frame to continue the mapping.
    FrameConsumed,
}

/// An error that can happen when trying to unmap a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    /// The given virtual address was not mapped to any physical address.
    NotMapped,
}

/// Map a physical address to a virtual address, allowing the kernel to access it.
/// The given rights and flags will be enforced by the Memory Management Unit (MMU)
/// of the system, and the physical address will be translated to the virtual address
/// when accessed by the kernel or the user.
pub fn map(
    table: &mut Table,
    virt: Virtual,
    phys: Physical,
    rights: Rights,
    flags: Flags,
) -> Result<(), MapError> {
    crate::target::mmu::map(table, virt, phys, rights, flags)
}

/// Unmap a virtual address, returning the physical address that was previously
/// mapped to it. If the virtual address was not mapped to any physical address,
/// this function will return an error.
pub fn unmap(table: &mut Table, virt: Virtual) -> Result<Physical, UnmapError> {
    crate::target::mmu::unmap(table, virt)
}

/// Translate a physical address to a virtual address. If the translation cannot be
/// done, this function will return `None`. This often happens when the physical
/// address cannot be mapped to a virtual address because the virtual address is
/// too small. This should only happen on 32-bit systems with more than 4 GiB of RAM,
/// which is not common.
#[must_use]
pub fn translate_physical(phys: Physical) -> Option<Virtual> {
    crate::target::mmu::translate_physical(phys)
}
