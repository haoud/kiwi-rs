use core::ops::{Deref, DerefMut};
use zerocopy::{FromBytes, IntoBytes};

use crate::{
    arch::thread::Thread,
    user::{self, ptr::Pointer},
};

/// An object that is stored in the userland address space. It is a structure
/// that holds a pointer to the object in the userland address space and a copy
/// of the object in the kernel address space. This allows us to read and write
/// the object in the userland address space safely.
#[derive(Debug)]
pub struct Object<'a, T: FromBytes + IntoBytes> {
    /// A pointer to the object in the userland address space.
    ptr: Pointer<'a, T>,

    /// A copy of the object in the kernel address space.
    inner: T,
}

impl<'a, T: FromBytes + IntoBytes> Object<'a, T> {
    /// Create an `Object` from the given pointer that resides in the userland
    /// memory. This function will read the object from the userland memory and
    /// store it in the `Object` struct.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and
    /// use the `copy_from` function to copy the object from the userland
    /// memory. This function is safe only if the pointer is valid and if the
    /// object in userland memory has exactly the same layout as the
    /// object in the kernel: otherwise, this function will cause undefined
    /// behavior.
    #[must_use]
    pub unsafe fn new(ptr: Pointer<'a, T>) -> Self {
        Self {
            inner: Self::read(&ptr),
            ptr,
        }
    }

    /// Create an `Object` from the given raw pointer that resides in the
    /// userland memory. This function will read the object from the userland
    /// memory and store it in the `Object` struct.
    ///
    /// If the pointer is not fully in the userland memory, it returns `None`.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and
    /// use the `copy_from` function to copy the object from the userland
    /// memory. This function is safe only if the pointer is valid and if the
    /// object in userland memory has exactly the same layout as the object in
    /// the kernel: otherwise, this function will cause undefined behavior.
    #[must_use]
    pub unsafe fn from_raw(thread: &'a Thread, ptr: *const T) -> Option<Self> {
        let user_ptr = Pointer::new(thread, ptr.cast_mut())?;
        Some(Self::new(user_ptr))
    }

    /// Manually update the object in the userland memory. This function will
    /// write the object back to the userland memory, so the object in the
    /// userland memory will be updated.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and
    /// use the `copy_from` function to copy the object from the userland
    /// memory. This function is safe if the pointer is valid and if the object
    /// in userland memory has exactly the same layout as the object in the
    /// kernel: otherwise, this function will cause undefined behavior.
    pub unsafe fn update(&mut self) {
        user::op::write(self.ptr.thread(), &raw const self.inner, self.ptr.inner());
    }

    /// Read the object from the userland memory and return it. It return a
    /// copy of the object and does not modify the object in the userland
    /// memory. This is advantageous to use this over using the `Object` struct
    /// if you do not need to modify the object in the userland +memory.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and
    /// use the `copy_from` function to copy the object from the userland
    /// memory. This function is safe if the pointer is valid and if the object
    /// in userland memory has exactly the same layout as the object in the
    /// kernel: otherwise, this function will cause undefined behavior.
    #[must_use]
    pub unsafe fn read(src: &Pointer<T>) -> T {
        let mut dst = core::mem::MaybeUninit::<T>::uninit();
        user::op::read(src.thread(), src.inner(), dst.as_mut_ptr());
        dst.assume_init()
    }

    /// Write the object to the userland memory. This function will write the
    /// object to the userland memory, so the object in the userland memory
    /// will be updated. This function is advantageous to use this over using
    /// the `Object` struct if you do not need to read the object from the
    /// userland memory, but only need to write it.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and
    /// use the `copy_to` function to copy the object to the userland memory.
    /// This function is safe if the pointer is valid and if the object in
    /// userland memory has exactly the same layout as the object in the
    /// kernel: otherwise, this function will cause undefined behavior.
    pub unsafe fn write(dst: &Pointer<T>, src: &T) {
        user::op::write(dst.thread(), src, dst.inner());
    }
}

impl<T: FromBytes + IntoBytes> Deref for Object<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: FromBytes + IntoBytes> DerefMut for Object<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
