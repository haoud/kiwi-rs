use core::ops::Deref;
use usize_cast::IntoUsize;

/// A trait to verify that an object is aligned
pub trait IsAligned {
    fn is_aligned(&self, align: usize) -> bool;
}

/// A structure that guarantees that its inner value is aligned to the
/// specified alignement.
///
/// # Note
/// This structure does not guarantees that the inner value will be aligned
/// to the specified alignement in memory ! It only guarantees that the value
/// itself is aligned to the given alignement
#[derive(Clone)]
pub struct Aligned<T: IsAligned, const N: usize>(T);

impl<T: IsAligned, const N: usize> Aligned<T, N> {
    pub const POWER_OF_TWO_ASSERT: () = assert!(N.is_power_of_two());

    /// Create a new aligned value
    ///
    /// # Panics
    /// Panic if the value is not aligned to the required boundary (`N`)
    #[must_use]
    pub fn new(inner: T) -> Self {
        assert!(inner.is_aligned(N));
        Self(inner)
    }

    /// Create a new aligned value without checking if the value
    /// is properly align.
    ///
    /// # Safety
    /// The caller must ensure that the value is properly aligned to
    /// the required alignment, otherwise the behavior is undefined.
    #[must_use]
    pub const unsafe fn new_unchecked(inner: T) -> Self {
        Self(inner)
    }

    /// Return a reference to the inner value
    #[must_use]
    pub const fn inner(&self) -> &T {
        &self.0
    }

    /// Return the alignment of the inner value
    #[must_use]
    pub const fn align(&self) -> usize {
        N
    }

    /// Return the inner value
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: IsAligned + Copy + Clone, const N: usize> Copy for Aligned<T, N> {}

impl<T: IsAligned, const N: usize> Deref for Aligned<T, N> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IsAligned for u8 {
    fn is_aligned(&self, align: usize) -> bool {
        (*self as usize & (align - 1)) == 0
    }
}

impl IsAligned for u16 {
    fn is_aligned(&self, align: usize) -> bool {
        (*self as usize & (align - 1)) == 0
    }
}

impl IsAligned for u32 {
    fn is_aligned(&self, align: usize) -> bool {
        (*self as usize & (align - 1)) == 0
    }
}

impl IsAligned for u64 {
    fn is_aligned(&self, align: usize) -> bool {
        ((*self).into_usize() & (align - 1)) == 0
    }
}

impl IsAligned for u128 {
    fn is_aligned(&self, align: usize) -> bool {
        (*self & (align as u128 - 1)) == 0
    }
}

impl IsAligned for usize {
    fn is_aligned(&self, align: usize) -> bool {
        (*self & (align - 1)) == 0
    }
}

impl<T> IsAligned for *const T {
    fn is_aligned(&self, align: usize) -> bool {
        (*self as usize & (align - 1)) == 0
    }
}
