//! This module defines types and functions for working with virtual and
//! physical addresses in a type-safe way.
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// The page size for the architecture. If the architecture supports multiple
/// page sizes, this should be the minimum page size allowed by the
/// architecture.
pub const PAGE_SIZE: usize = crate::arch::target::addr::PAGE_SIZE;

/// A physical address space. This is used to represent physical addresses and
/// to distinguish between different types of physical addresses (e.g. DMA
/// addresses, high memory addresses...).
pub trait PhysicalSpace: Copy {
    const MIN: usize;
    const MAX: usize;
}

/// A virtual address space. This is used to distinguish between kernel and
/// user addresses, and to provide type safety when working with addresses.
pub trait VirtualSpace: Copy {
    const MIN: usize;
    const MAX: usize;
}

/// All physical memory. This is used to represent physical addresses, and the
/// architecture-dependent code is responsible for implementing the `PhysicalSpace`
/// trait for this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllMemory {}

/// Kernel virtual address space, only accessible from kernel mode. The
/// architecture-dependent code is responsible for implementing the
/// `VirtualSpace` trait for this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kernel {}

/// User virtual address space, only accessible from user mode and by the
/// kernel when SMAP (Supervisor Mode Access Prevention) is disabled. The
/// architecture-dependent code is responsible for implementing the
/// `VirtualSpace` trait for this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User {}

/// A virtual address in the address space `T`. This type ensures that virtual
///  addresses are always valid in their respective address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Virtual<T>(usize, core::marker::PhantomData<T>);

impl<T: VirtualSpace> Virtual<T> {
    /// Try to create a new virtual address from a raw address. This will
    /// return `None` if the address is not in the virtual address space
    /// of `T`.
    #[must_use]
    pub const fn try_new(addr: usize) -> Option<Self> {
        #[allow(clippy::absurd_extreme_comparisons)]
        if addr >= T::MIN && addr <= T::MAX {
            Some(Self(addr, core::marker::PhantomData))
        } else {
            None
        }
    }

    /// Get the misalignment of the virtual address with respect to the given
    /// alignment. This will return a value in the range `[0, align)`, where
    /// `align` must be a power of two.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two.
    #[must_use]
    pub const fn misalign(&self, align: usize) -> usize {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        self.0 & (align - 1)
    }

    /// Check if the virtual address is aligned to the given alignment. This
    /// will return `true` if the address is aligned, and `false` otherwise.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two.
    #[must_use]
    pub const fn is_aligned(&self, align: usize) -> bool {
        self.misalign(align) == 0
    }

    /// Align the virtual address up to the nearest multiple of `align`. If the
    /// address is already aligned, it will be returned unchanged.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two, or if the
    /// resulting address would overflow the virtual address space of `T`.
    #[must_use]
    pub const fn align_up(&self, align: usize) -> Self {
        Self::try_new(align_up(self.0, align))
            .expect("Resulting address is not in the address space")
    }

    /// Align the virtual address down to the nearest multiple of `align`. If the
    /// address is already aligned, it will be returned unchanged.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two, or if the
    /// resulting address would overflow the virtual address space of `T`.
    #[must_use]
    pub const fn align_down(&self, align: usize) -> Self {
        Self::try_new(align_down(self.0, align))
            .expect("Resulting address is not in the address space")
    }

    /// Try to align the virtual address down to the nearest multiple of
    /// `align`. If the address is already aligned, it will be returned
    /// unchanged. Return `None` if `align` is not a power of two.
    #[must_use]
    pub const fn try_align_down(&self, align: usize) -> Option<Self> {
        if let Some(addr) = try_align_down(self.0, align) {
            Self::try_new(addr)
        } else {
            None
        }
    }

    /// Try to align the virtual address up to the nearest multiple of `align`.
    /// If the address is already aligned, it will be returned unchanged. Return
    /// `None` if one or more of the following conditions are true:
    /// - The resulting address would be outside the virtual address space of `T`.
    /// - `align` is not a power of two.
    #[must_use]
    pub const fn try_align_up(&self, align: usize) -> Option<Self> {
        if let Some(addr) = try_align_up(self.0, align) {
            Self::try_new(addr)
        } else {
            None
        }
    }

    /// Convert the virtual address to a mutable pointer of type `U`.
    #[must_use]
    pub const fn as_mut_ptr<U>(&self) -> *mut U {
        self.0 as *mut U
    }

    /// Convert the virtual address to a const pointer of type `U`.
    #[must_use]
    pub const fn as_ptr<U>(&self) -> *const U {
        self.0 as *const U
    }

    /// Convert the virtual address to a raw address.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }
}

impl Virtual<User> {
    /// Create a new user virtual address from a raw address.
    ///
    /// # Panics
    /// This function will panic if the address is not in the user virtual
    /// address space.
    #[must_use]
    pub const fn new(addr: usize) -> Self {
        Self::try_new(addr).expect("Address is not in the user virtual address space")
    }
}

impl Virtual<Kernel> {
    /// Create a new kernel virtual address from a raw address.
    ///
    /// # Panics
    /// This function will panic if the address is not in the kernel virtual
    /// address space.
    #[must_use]
    pub const fn new(addr: usize) -> Self {
        Self::try_new(addr).expect("Address is not in the kernel virtual address space")
    }
}

impl<T: VirtualSpace> From<Virtual<T>> for usize {
    fn from(addr: Virtual<T>) -> Self {
        addr.0
    }
}

impl<T: VirtualSpace> core::fmt::Binary for Virtual<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Binary::fmt(&self.0, f)
    }
}

impl<T: VirtualSpace> core::fmt::Octal for Virtual<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Octal::fmt(&self.0, f)
    }
}

impl<T: VirtualSpace> core::fmt::LowerHex for Virtual<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl<T: VirtualSpace> core::fmt::UpperHex for Virtual<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl<T: VirtualSpace> core::fmt::Pointer for Virtual<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl<T: VirtualSpace> core::fmt::Display for Virtual<T> {
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

impl<T: VirtualSpace> Add<usize> for Virtual<T> {
    type Output = Self;

    /// Adds an offset to a virtual address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// virtual address in the given address space, or if the addition would
    /// overflow.
    fn add(self, rhs: usize) -> Self::Output {
        Self::try_new(
            self.0
                .checked_add(rhs)
                .expect("Attempt to add with overflow"),
        )
        .expect("Resulting address is not a valid virtual address in the given address space")
    }
}

impl<T: VirtualSpace> Sub<usize> for Virtual<T> {
    type Output = Self;

    /// Subtracts an offset from a virtual address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// virtual address in the given address space, or if the subtraction
    /// would underflow.
    fn sub(self, rhs: usize) -> Self::Output {
        Self::try_new(
            self.as_usize()
                .checked_sub(rhs)
                .expect("Attempt to subtract with underflow"),
        )
        .expect("Resulting address is not a valid virtual address in the given address space")
    }
}

impl<T: VirtualSpace> AddAssign<usize> for Virtual<T> {
    /// Adds an offset to a virtual address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// virtual address in the given address space, or if the addition
    /// would overflow.
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl<T: VirtualSpace> SubAssign<usize> for Virtual<T> {
    /// Subtracts an offset from a virtual address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// virtual address in the given address space, or if the subtraction
    /// would underflow.
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

/// A physical address in the physical address space `T`. This type ensures
/// that physical addresses are always valid in their respective address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Physical<T>(usize, core::marker::PhantomData<T>);

impl<T: PhysicalSpace> Physical<T> {
    /// Try to create a new physical address from a raw address. This will
    /// return `None` if the address is not a valid physical address in the
    /// physical address space of `T`.
    #[must_use]
    pub const fn try_new(addr: usize) -> Option<Self> {
        #[allow(clippy::absurd_extreme_comparisons)]
        if addr >= T::MIN && addr <= T::MAX {
            Some(Self(addr, core::marker::PhantomData))
        } else {
            None
        }
    }

    /// Get the misalignment of the physical address with respect to the given
    /// alignment. This will return a value in the range `[0, align)`, where
    /// `align` must be a power of two.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two.
    #[must_use]
    pub const fn misalign(&self, align: usize) -> usize {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        self.0 & (align - 1)
    }

    /// Check if the physical address is aligned to the given alignment. This
    /// will return `true` if the address is aligned, and `false` otherwise.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two.
    #[must_use]
    pub const fn is_aligned(&self, align: usize) -> bool {
        self.misalign(align) == 0
    }

    /// Align the physical address up to the nearest multiple of `align`. If the
    /// address is already aligned, it will be returned unchanged.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two, or if the
    /// resulting address would overflow the physical address space of `T`.
    #[must_use]
    pub const fn align_up(&self, align: usize) -> Self {
        Self::try_new(align_up(self.0, align))
            .expect("Resulting address is not in the given physical address space")
    }

    /// Align the physical address down to the nearest multiple of `align`. If the
    /// address is already aligned, it will be returned unchanged.
    ///
    /// # Panics
    /// This function will panic if `align` is not a power of two, or if the
    /// resulting address would overflow the physical address space of `T`.
    #[must_use]
    pub const fn align_down(&self, align: usize) -> Self {
        Self::try_new(align_down(self.0, align))
            .expect("Resulting address is not in the given physical address space")
    }

    /// Try to align the physical address down to the nearest multiple of
    /// `align`. If the address is already aligned, it will be returned
    /// unchanged. Return `None` if `align` is not a power of two.
    #[must_use]
    pub const fn try_align_down(&self, align: usize) -> Option<Self> {
        if let Some(addr) = try_align_down(self.0, align) {
            Self::try_new(addr)
        } else {
            None
        }
    }

    /// Try to align the physical address up to the nearest multiple of `align`.
    /// If the address is already aligned, it will be returned unchanged. Return
    /// `None` if one or more of the following conditions are true:
    /// - The resulting address would be outside the physical address space of `T`.
    /// - `align` is not a power of two.
    #[must_use]
    pub const fn try_align_up(&self, align: usize) -> Option<Self> {
        if let Some(addr) = try_align_up(self.0, align) {
            Self::try_new(addr)
        } else {
            None
        }
    }

    /// Convert the physical address to a raw address.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }
}

impl Physical<AllMemory> {
    /// Create a new physical address from a raw address.
    ///
    /// # Panics
    /// This function will panic if the address is not a valid physical
    /// address.
    #[must_use]
    pub const fn new(addr: usize) -> Self {
        Self::try_new(addr).expect("Address is not a valid physical address")
    }
}

impl<T: PhysicalSpace> From<Physical<T>> for usize {
    fn from(addr: Physical<T>) -> Self {
        addr.0
    }
}

impl<T: PhysicalSpace> core::fmt::Binary for Physical<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Binary::fmt(&self.0, f)
    }
}

impl<T: PhysicalSpace> core::fmt::Octal for Physical<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Octal::fmt(&self.0, f)
    }
}

impl<T: PhysicalSpace> core::fmt::LowerHex for Physical<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl<T: PhysicalSpace> core::fmt::UpperHex for Physical<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl<T: PhysicalSpace> core::fmt::Pointer for Physical<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl<T: PhysicalSpace> core::fmt::Display for Physical<T> {
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

impl<T: PhysicalSpace> Add<usize> for Physical<T> {
    type Output = Self;

    /// Adds an offset to a physical address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// physical address in the given address space, or if the addition would
    /// overflow.
    fn add(self, rhs: usize) -> Self::Output {
        Self::try_new(
            self.0
                .checked_add(rhs)
                .expect("Attempt to add with overflow"),
        )
        .expect("Resulting address is not a valid physical address in the given address space")
    }
}

impl<T: PhysicalSpace> Sub<usize> for Physical<T> {
    type Output = Self;

    /// Subtracts an offset from a physical address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// physical address in the given address space, or if the subtraction
    /// would underflow.
    fn sub(self, rhs: usize) -> Self::Output {
        Self::try_new(
            self.as_usize()
                .checked_sub(rhs)
                .expect("Attempt to subtract with underflow"),
        )
        .expect("Resulting address is not a valid physical address in the given address space")
    }
}

impl<T: PhysicalSpace> AddAssign<usize> for Physical<T> {
    /// Adds an offset to a physical address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// physical address in the given address space, or if the addition
    /// would overflow.
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl<T: PhysicalSpace> SubAssign<usize> for Physical<T> {
    /// Subtracts an offset from a physical address.
    ///
    /// # Panics
    /// This function will panic if the resulting address is not a valid
    /// physical address in the given address space, or if the subtraction
    /// would underflow.
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

/// Align the given address `addr` down to the nearest multiple of `align`. If
/// the address is already aligned, it will be returned unchanged.
///
/// # Panics
/// This function will panic if `align` is not a power of two.
#[inline]
const fn align_down(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two(), "Alignment must be a power of two");
    addr & !(align - 1)
}

/// Align the given address `addr` up to the nearest multiple of `align`. If
/// the address is already aligned, it will be returned unchanged.
///
/// # Panics
/// This function will panic if `align` is not a power of two, or if the
/// resulting address would overflow an `usize`.
#[inline]
const fn align_up(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two(), "Alignment must be a power of two");
    addr.checked_add(align - 1)
        .expect("Address overflow when aligning up")
        & !(align - 1)
}

/// Try to align the given address `addr` down to the nearest multiple of
/// `align`. If the address is already aligned, it will be returned unchanged.
/// Return `None` if `align` is not a power of two.
#[inline]
const fn try_align_down(addr: usize, align: usize) -> Option<usize> {
    if align.is_power_of_two() {
        Some(addr & !(align - 1))
    } else {
        None
    }
}

/// Try to align the given address `addr` up to the nearest multiple of
/// `align`. If the address is already aligned, it will be returned
/// unchanged. Return `None` if `align` is not a power of two, or if
/// the resulting address would overflow an `usize`.
#[inline]
const fn try_align_up(addr: usize, align: usize) -> Option<usize> {
    if align.is_power_of_two()
        && let Some(addr) = addr.checked_add(align - 1)
    {
        return Some(addr & !(align - 1));
    }

    None
}
