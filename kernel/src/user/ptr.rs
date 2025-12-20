use crate::arch::riscv64::addr::{Virtual, virt::User};

/// This structure encapsulate a pointer to an object in the userland memory:
/// this structure guarantees that the pointer is in the userland memory.
///
/// # Data Races
/// Contrary to the kernel, data races are allowed in the userland memory. This
/// is because multiple tasks can share the same memory space in the userland
/// memory, and therefore can pass at the same time the same pointer to the
/// kernel. This is the userland programmer responsibility to ensure that there
/// is no data races in their program: the kernel cannot ensure this because
/// not all user applications are written in Rust and follow the Rust memory
/// safety rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer<T> {
    inner: *mut T,
}

impl<T> Pointer<T> {
    /// Tries to create a new user pointer. Returns `None` if the given pointer
    /// is not fully in the userland memory. This is equivalent to calling
    /// `Pointer::array` with a length of 1.
    #[must_use]
    pub fn new(ptr: *mut T) -> Option<Self> {
        Self::array(ptr, 1)
    }

    /// Tries to create a new user pointer to an array of `len` elements. Returns
    /// `None` if the given pointer is not fully in the userland memory.
    #[must_use]
    pub fn array(ptr: *mut T, len: usize) -> Option<Self> {
        let start = Virtual::<User>::try_new(ptr.cast::<u8>().addr());
        let end = Virtual::<User>::try_new(
            ptr.cast::<u8>()
                .wrapping_add(core::mem::size_of::<T>() * len)
                .addr(),
        );

        // Check that the whole range is in the userland address space and
        // that the start address is lower than the end address (to prevent
        // overflow that would make both addresses valid, but the range
        // invalid).
        if let (Some(start), Some(end)) = (start, end)
            && start <= end
        {
            return Some(Self { inner: ptr });
        }
        None
    }

    /// Get the raw pointer to the object in the userland memory.
    #[must_use]
    pub const fn inner(&self) -> *mut T {
        self.inner
    }
}

impl<T> core::fmt::Display for Pointer<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:016x}", self.inner.addr())
    }
}
