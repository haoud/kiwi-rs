pub use crate::arch::target::mmu::{PAGE_SHIFT, PAGE_SIZE};
use crate::arch::target::{
    addr::{self, Frame4Kib, Physical, Virtual, virt::Kernel},
    mmu::Table,
};
use bitflags::bitflags;
use usize_cast::IntoUsize;

pub trait Align {
    /// Assume that the value is a address and return the address aligned to
    /// the nearest previous page. If the address is already aligned to the
    /// page size, the address will not be changed.
    #[must_use]
    fn page_align_down(&self) -> Self;

    /// Assume that the value is a size in bytes and return the number of
    /// pages that it represents. If the value is not a multiple of the page
    /// size, the result will be rounded up to the nearest page.
    #[must_use]
    fn page_count_down(&self) -> usize;

    /// Assume that the value is a address and return the address aligned to
    /// the nearest next page. If the address is already aligned to the page
    /// size, the address will not be changed.
    #[must_use]
    fn page_align_up(&self) -> Self;

    /// Assume that the value is a size in bytes and return the number of pages
    /// that it represents. If the value is not a multiple of the page size,
    /// the result will be rounded up to the nearest page.
    #[must_use]
    fn page_count_up(&self) -> usize;
}

impl Align for usize {
    fn page_count_down(&self) -> usize {
        self / PAGE_SIZE
    }

    fn page_count_up(&self) -> usize {
        self.div_ceil(PAGE_SIZE)
    }

    fn page_align_down(&self) -> Self {
        self & !(PAGE_SIZE - 1)
    }

    fn page_align_up(&self) -> Self {
        (self + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
    }
}

impl Align for u64 {
    fn page_count_down(&self) -> usize {
        self.into_usize() / PAGE_SIZE
    }

    fn page_count_up(&self) -> usize {
        self.into_usize().div_ceil(PAGE_SIZE)
    }

    fn page_align_down(&self) -> Self {
        self & !(PAGE_SIZE as u64 - 1)
    }

    fn page_align_up(&self) -> Self {
        (self + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1)
    }
}

bitflags! {
    /// A set of rights that can be granted to a memory region. These rights
    /// are used to control the access to the memory region, and are enforced
    /// by the Memory Management Unit (MMU) of the system. For some
    /// architectures, some of these rights may not be supported (e.g the
    /// `EXECUTE` right on some x86 systems) or may be implicit (e.g the `READ`
    /// is always granted on x86 systems if the page is present).
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Rights: u32 {
        const USER = 1 << 0;
        const READ = 1 << 1;
        const WRITE = 1 << 2;
        const EXECUTE = 1 << 3;

        const RX = Self::READ.bits() | Self::EXECUTE.bits();
        const RW = Self::READ.bits() | Self::WRITE.bits();
        const RWX = Self::RW.bits() | Self::EXECUTE.bits();
        const RWXU = Self::RWX.bits() | Self::USER.bits();
    }

    /// A set of flags that can be used to control the behavior
    /// of a memory region.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u32 {
        /// The memory region is global and should not be flushed from the
        /// TLB when changing the address space. This must be only used if a
        /// page is shared between all address spaces. Otherwise, it may lead
        /// to security issues or strange bugs that will be very, very hard
        /// to debug.
        const GLOBAL = 1 << 0;
    }
}

/// An error that can happen when trying to map a
/// physical address to a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// A invalid combination of flags was given.
    InvalidFlagsCombination,

    /// The frame is not aligned to the required size. The frame must be
    /// aligned to either 4 KiB, 2 MiB or 1 GiB, depending on the flags given.
    FrameNotAligned,

    /// The given frame was already mapped to another virtual address.
    /// The caller should unmap the frame from the other virtual address
    /// before trying to map it again.
    AlreadyMapped,

    /// The kernel ran out of memory while trying to map the frame. This
    /// can happen if the kernel needs to allocate an intermediate page table
    /// to map the frame, but there is no memory available to do so.
    OutOfMemory,
}

/// An error that can happen when trying to unmap a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    /// The given virtual address was not mapped to any physical address.
    NotMapped,
}

/// Map a physical address to a virtual address, allowing the kernel to
/// access it. The given rights and flags will be enforced by the Memory
/// Management Unit (MMU) of the system, and the physical address will be
/// translated to the virtual address when accessed by the kernel or the user.
///
/// # Errors
/// For an exhaustive list of errors that can happen when trying to map a
/// physical address to a virtual address, see the [`MapError`] enum.
pub fn map<T: addr::virt::Type>(
    table: &mut Table,
    virt: Virtual<T>,
    frame: Frame4Kib,
    rights: Rights,
    flags: Flags,
) -> Result<(), MapError> {
    crate::arch::target::mmu::map(table, virt, frame, rights, flags)
}

/// Unmap a virtual address, returning the physical address that was
/// previously mapped to it. If the virtual address was not mapped to
/// any physical address, this function will return an error.
///
/// # Errors
/// For an exhaustive list of errors that can happen when trying to unmap a
/// virtual address, see the [`UnmapError`] enum.
pub fn unmap<T: addr::virt::Type>(
    table: &mut Table,
    virt: Virtual<T>,
) -> Result<Physical, UnmapError> {
    crate::arch::target::mmu::unmap(table, virt)
}

/// Translate a physical address to a virtual address. If the translation
/// cannot be done, this function will return `None`. This often happens when
/// the physical address cannot be mapped to a virtual address because the
/// virtual address is too small. This should only happen on 32-bit systems
/// with more than 4 GiB of RAM, which is not common.
#[must_use]
pub fn translate_physical(phys: impl Into<Physical>) -> Option<Virtual<Kernel>> {
    crate::arch::target::mmu::translate_physical(phys)
}
