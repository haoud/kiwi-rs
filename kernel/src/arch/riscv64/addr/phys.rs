use crate::{arch::mmu, utils::align::IsAligned};
use core::ops::{Add, AddAssign, Sub, SubAssign};
use usize_cast::IntoUsize;

use super::{Frame1Gib, Frame2Mib, Frame4Kib};

/// A physical address in the RISC-V architecture. A physical address is a
/// direct mapping to the physical memory of the system (RAM, ROM, etc). Since
/// Kiwi always uses Sv39 paging, the physical address cannot directly be used
/// to access memory. Instead, it must be translated to a virtual address with7
/// the help of the MMU.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Physical(usize);

impl Physical {
    /// A physical address of 0.
    pub const ZERO: Self = Self(0);

    /// The maximum physical address that can be represented on a RISC-V
    /// system. This does not necessarily mean that all of this address space
    /// is available for use but rather the size of the memory bus. Therefore,
    /// no valid physical address can be greater than this value.
    pub const MAX: Self = Self(0x0000_0FFF_FFFF_FFFF);

    /// Create a new `Physical` address.
    ///
    /// # Panics
    /// This function will panic if the address is greater than the maximum
    /// physical address (as defined by [`MAX`]).
    #[must_use]
    pub const fn new(addr: usize) -> Self {
        assert!(addr <= Self::MAX.0, "Physical address out of bounds");
        Self(addr)
    }

    /// Create a new `Physical` address without checking if the address is
    /// valid or not.
    ///
    /// # Safety
    /// The address must be less or equal to [`MAX`]. If this is not the case,
    /// the behavior of futher methods call is undefined.
    #[must_use]
    pub const unsafe fn new_unchecked(addr: usize) -> Self {
        Self(addr)
    }

    /// Attempt to create a new `Physical` address. If the address is greater
    /// than the maximum physical address (as defined by [`MAX`]), then `None`
    /// is returned.
    #[must_use]
    pub const fn try_new(addr: usize) -> Option<Self> {
        if addr <= Self::MAX.0 {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Create a new `Physical` address. If the address is greater than the
    /// maximal physical address (as defined by [`MAX`]), then the address is
    /// truncated in order to be less than [`MAX`].
    #[must_use]
    pub const fn new_truncate(addr: usize) -> Self {
        Self(addr & 0x0000_0FFF_FFFF_FFFF)
    }

    /// Create an 0 physical address.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Return the address as a `usize`.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Return the address as a `u64`.
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    /// Check if the address is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Align down the physical address to the given alignment. If the physical
    /// address is already aligned to the given alignment, the address will not
    /// be changed.
    ///
    /// # Panics
    /// Panic if the alignement is not a power of two
    #[must_use]
    pub const fn align_down(self, align: usize) -> Self {
        assert!(align.is_power_of_two());
        Self(self.0 & !(align - 1))
    }

    /// Align up the physical address to the given alignment. The alignment
    /// must be a power of two, otherwise the result will be incorrect. If the
    /// physical address is already aligned to the given alignment, the address
    /// will not be changed.
    ///
    /// # Panics
    /// Panic if the alignement is not a power of two.
    #[must_use]
    pub const fn align_up(self, align: usize) -> Self {
        assert!(align.is_power_of_two());
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Verify if the physical address is aligned to the given alignment.
    ///
    /// # Panics
    /// Panic if the alignement is not a power of two.
    #[must_use]
    pub const fn is_aligned_to(self, align: usize) -> bool {
        assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }

    /// Align the address down to the nearest page boundary. If the address is
    /// already page aligned, then it is returned as is.
    #[must_use]
    pub const fn page_align_down(&self) -> Self {
        Self(self.0 & !(mmu::PAGE_SIZE - 1))
    }

    /// Align the address up to the nearest page boundary. If the address is
    /// already page aligned, then it is returned as is.
    ///
    /// # Panics
    /// This function will panic if the resulting address is greater than the
    /// maximum physical address (as defined by [`MAX`]).
    #[must_use]
    pub const fn page_align_up(&self) -> Self {
        Self::new((self.0 + mmu::PAGE_SIZE - 1) & !(mmu::PAGE_SIZE - 1))
    }

    /// Check if the address is page aligned.
    #[must_use]
    pub const fn is_page_aligned(&self) -> bool {
        self.0.is_multiple_of(mmu::PAGE_SIZE)
    }

    /// Convert the physical address to a frame index.
    #[must_use]
    pub const fn frame_idx(&self) -> usize {
        self.0 / mmu::PAGE_SIZE
    }

    /// Convert the physical address to a 4KiB frame. If the address is not
    /// aligned to a 4KiB frame, the address will be truncated to the nearest
    /// lower 4KiB frame.
    #[must_use]
    pub fn into_4kib_frame_truncate(self) -> Frame4Kib {
        Frame4Kib::new(Physical::new(self.0 / Frame4Kib::SIZE))
    }

    /// Convert the physical address to a 2MiB frame. If the address is not
    /// aligned to a 2MiB frame, the address will be truncated to the nearest
    /// lower 2MiB frame.
    #[must_use]
    pub fn into_2mib_frame_truncate(self) -> Frame2Mib {
        Frame2Mib::new(Physical::new(self.0 / Frame2Mib::SIZE))
    }

    /// Convert the physical address to a 1GiB frame. If the address is not
    /// aligned to a 1GiB frame, the address will be truncated to the nearest
    /// lower 1GiB frame.
    #[must_use]
    pub fn into_1gib_frame_truncate(self) -> Frame1Gib {
        Frame1Gib::new(Physical::new(self.0 / Frame1Gib::SIZE))
    }
}

impl TryFrom<usize> for Physical {
    type Error = ();

    fn try_from(addr: usize) -> Result<Self, Self::Error> {
        Self::try_new(addr).ok_or(())
    }
}

impl TryFrom<u64> for Physical {
    type Error = ();

    fn try_from(addr: u64) -> Result<Self, Self::Error> {
        Self::try_new(addr.into_usize()).ok_or(())
    }
}

impl From<Physical> for usize {
    fn from(addr: Physical) -> Self {
        addr.0
    }
}

impl From<Physical> for u64 {
    fn from(addr: Physical) -> Self {
        addr.as_u64()
    }
}

impl Add<Physical> for Physical {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}

impl Add<usize> for Physical {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl Add<u64> for Physical {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self::new(self.0 + rhs.into_usize())
    }
}

impl Sub<Physical> for Physical {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}

impl Sub<usize> for Physical {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

impl Sub<u64> for Physical {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Self::new(self.0 - rhs.into_usize())
    }
}

impl AddAssign<Physical> for Physical {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl AddAssign<usize> for Physical {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl AddAssign<u64> for Physical {
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl SubAssign<Physical> for Physical {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl SubAssign<usize> for Physical {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

impl SubAssign<u64> for Physical {
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl IsAligned for Physical {
    /// Check if the physical address is aligned to the given alignment. The
    /// given alignment must be a power of two, otherwise the result will be
    /// incorrect.
    fn is_aligned(&self, align: usize) -> bool {
        self.is_aligned_to(align)
    }
}
