use core::iter::Step;

use crate::arch::{
    self,
    addr::{AllMemory, PAGE_SIZE, Physical},
};

/// A frame index, which is the sequential number of a frame in the physical
/// address space, calculated by dividing the physical address by the page
/// size.
///
/// Each frame has a unique index, and the index of a frame can be used to
/// calculate its physical address by multiplying the index by the page size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Index(usize);

impl Index {
    /// Creates a new frame index with the given value.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the value of this frame index.
    #[must_use]
    pub const fn value(&self) -> usize {
        self.0
    }

    /// Returns the physical address of the frame corresponding to this index.
    ///
    /// # Panics
    /// Panics if the calculated physical address would not be a valid physical
    /// address or would overflow.
    #[must_use]
    pub const fn physical(&self) -> Physical<AllMemory> {
        Physical::try_new(self.0 * PAGE_SIZE).expect("Frame index overflow")
    }
}

impl From<Index> for usize {
    fn from(index: Index) -> Self {
        index.0
    }
}

impl From<Frame> for Index {
    fn from(frame: Frame) -> Self {
        frame.index()
    }
}

impl<T: arch::addr::PhysicalSpace> From<Physical<T>> for Index {
    fn from(phys: Physical<T>) -> Self {
        Self::new(phys.as_usize() / PAGE_SIZE)
    }
}

/// A physical frame, guaranteed to be aligned to the page boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Frame(Physical<AllMemory>);

impl Frame {
    /// Creates a new frame with the given physical address.
    ///
    /// # Panics
    /// Panics if the address is not aligned to the page size.
    #[must_use]
    pub const fn new(addr: Physical<AllMemory>) -> Self {
        assert!(addr.is_aligned(PAGE_SIZE));
        Self(addr)
    }

    /// Creates a new frame with the given physical address, aligning it down
    /// to the nearest page boundary if necessary.
    #[must_use]
    pub const fn new_align_down(addr: Physical<AllMemory>) -> Self {
        Self(addr.align_down(PAGE_SIZE))
    }

    /// Creates a new frame with the given physical address, aligning it up to
    /// the nearest page boundary if necessary.
    ///
    /// # Panics
    /// Panics if the aligned address would overflow.
    #[must_use]
    pub const fn new_align_up(addr: Physical<AllMemory>) -> Self {
        Self(addr.align_up(PAGE_SIZE))
    }

    /// Returns the physical address of this frame.
    #[must_use]
    pub const fn physical(&self) -> Physical<AllMemory> {
        self.0
    }

    /// Returns the frame index of this frame.
    #[must_use]
    pub const fn index(&self) -> Index {
        Index::new(self.0.as_usize() / PAGE_SIZE)
    }

    /// Returns the physical address of this frame as a `usize`.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0.as_usize()
    }
}

impl From<Frame> for usize {
    fn from(frame: Frame) -> Self {
        usize::from(frame.0)
    }
}

impl core::fmt::Binary for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Binary::fmt(&self.0, f)
    }
}

impl core::fmt::Octal for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Octal::fmt(&self.0, f)
    }
}

impl core::fmt::LowerHex for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl core::fmt::UpperHex for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl core::fmt::Pointer for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(usize::from(*self) as *const ()), f)
    }
}

impl core::fmt::Display for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if cfg!(target_pointer_width = "64") {
            write!(f, "0x{:016x}", self.0)
        } else if cfg!(target_pointer_width = "32") {
            write!(f, "0x{:08x}", self.0)
        } else {
            unreachable!("Unsupported target pointer width")
        }
    }
}

impl Step for Frame {
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        if start > end {
            return (0, None);
        }

        let steps = (usize::from(*end) - usize::from(*start)) / PAGE_SIZE;
        (steps, Some(steps))
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let offset = count.checked_mul(PAGE_SIZE)?;
        let addr = usize::from(start.0).checked_add(offset)?;
        Some(Self(Physical::try_new(addr)?))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let offset = count.checked_mul(PAGE_SIZE)?;
        let addr = usize::from(start.0).checked_sub(offset)?;
        Some(Self(Physical::try_new(addr)?))
    }
}
