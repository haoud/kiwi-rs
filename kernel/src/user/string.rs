use crate::user::{self, ptr::Pointer};

/// A string that is stored in the userland address space. It is a structure
/// that are created by the rust syscall wrapper and passed to the kernel, so
/// the kernel can then fetch the string from the userland address space.
///
/// We cannot directly pass a `String` to the kernel, because the layout of an
/// `String` is unspecified and may change between different versions of Rust.
/// Therefore, we use this custom structure that has a fixed layout, allowing
/// us to safely read it from the userland address space in the kernel.
#[repr(C)]
pub struct RawString {
    pub data: *mut u8,
    pub len: usize,
}

/// Represents a UTF-8 string that is stored in the userland address space.
/// This structure is similar to the [`RawString`] structure, but it more
/// convenient to use in the kernel as it make some guarantees about the
/// pointer that the [`RawString`] structure cannot make.
#[derive(Debug)]
pub struct String {
    data: Pointer<u8>,
    len: usize,
}

impl String {
    /// The maximum length of a string that can be fetched from the userland
    /// address space. This limit is imposed to prevent the kernel from trying
    /// to fetch excessively long strings that could lead to denial of service
    /// attacks or exhaust kernel memory.
    pub const MAX_LEN: usize = 4096;

    /// Creates a new user string from a raw pointer and a length. This
    /// function does not copy the string from the userland address space to
    /// the kernel address space, but simply create a new string with an user
    /// pointer to the string in the userland address space and the length of
    /// the string.
    ///
    /// If the pointer is invalid or if the whole string does not reside in
    /// the userland address space, then this function will return `None`.
    #[must_use]
    pub fn new(ptr: *mut u8, len: usize) -> Option<Self> {
        let data = Pointer::array(ptr, len)?;
        Some(Self { data, len })
    }

    /// Creates a new user string from a string from a syscall. This function
    /// does not copy the string from the userland address space to the kernel
    /// address space, but simply create a new string with an user pointer to
    /// the string in the userland address space and the length of the string.
    ///
    /// If the pointer contained in the syscall string is invalid or if the
    /// whole string does not reside in the userland address space, then this
    /// function will return `None`.
    #[must_use]
    pub fn from_raw(str: &RawString) -> Option<Self> {
        let data = Pointer::array(str.data, str.len)?;
        Some(Self { data, len: str.len })
    }

    /// Fetches a string from the userland address space. This function will
    /// copy the string from the userland address space to the kernel address
    /// space and return it as an `String`. All modifications to the returned
    /// string will not affect the userland string.
    ///
    /// # Errors
    /// This function will return an error if any of the following conditions
    /// are met (see the [`FetchError`] enum for more details):
    /// - The user pointer is invalid: not mapped, not readable or not in the
    ///   userland address space
    /// - The string is longer than [`Self::MAX_LEN`] bytes
    /// - The string is not valid UTF-8
    pub fn fetch(&self) -> Result<alloc::string::String, FetchError> {
        // Check if the string is too long to be handled by the kernel.
        if self.len > Self::MAX_LEN {
            return Err(FetchError::StringTooLong);
        }

        // Allocate a vector with the same size as the string and prepare the copy
        let mut vector = alloc::vec::Vec::with_capacity(self.len);
        let dst = vector.as_mut_ptr();
        let src = self.data.inner();
        let len = self.len;

        // SAFETY: This is safe because we checked that the string is entirely
        // in the userland address space and that the string is not too long
        // to be handled by the kernel. Data race are permitted here because
        // the string resides in the userland address space and the kernel
        // cannot prevent data races in the userland address space: it is the
        // responsability of the user program. We also set the length of the
        // vector after the copy to the correct length.
        unsafe {
            user::op::copy_from(src, dst, len);
            vector.set_len(len);
        }

        Ok(alloc::string::String::from_utf8(vector)?)
    }
}

/// An enum that represents an error that can occur when fetching an string from
/// the userland address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchError {
    /// The pointer is invalid: it may be not mapped, not accessible in read mode or not
    /// in the userland address space.
    InvalidMemory,

    /// The string is longer than [`Self::MAX_LEN`] bytes.
    StringTooLong,

    /// The string is not a valid UTF-8 string.
    StringNotUtf8,
}

impl From<alloc::string::FromUtf8Error> for FetchError {
    fn from(_: alloc::string::FromUtf8Error) -> Self {
        Self::StringNotUtf8
    }
}
