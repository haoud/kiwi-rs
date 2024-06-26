use crate::{arch::mmu, utils::align::IsAligned};
use core::marker::PhantomData;

/// A kernel virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kernel;

/// A user virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User;

/// The type of a virtual address. Either `Kernel` or `User`.
pub trait Type: Copy {}
impl Type for Kernel {}
impl Type for User {}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Virtual<T: Type> {
    _phantom: PhantomData<T>,
    addr: usize,
}

impl<T: Type> Virtual<T> {
    /// Create a new virtual address without performing any checks.
    ///
    /// # Safety
    /// The caller must ensure that the virtual address is valid according to
    /// the requested variant (`KERNEL` or `USER`)
    #[must_use]
    pub const fn new_unchecked(addr: usize) -> Self {
        Self {
            _phantom: PhantomData,
            addr,
        }
    }

    /// Create a new virtual address from a pointer without performing any
    /// checks.
    ///
    /// # Safety
    /// The caller must ensure that the virtual address is valid according to
    /// the requested variant (`KERNEL` or `USER`)
    #[must_use]
    pub fn from_ptr_unchecked<P>(ptr: *const P) -> Self {
        Self::new_unchecked(ptr as usize)
    }

    /// Return the physical address as a mutable pointer.
    #[must_use]
    pub const fn as_mut_ptr<P>(&self) -> *mut P {
        self.addr as *mut P
    }

    /// Return the physical address as a const pointer.
    #[must_use]
    pub const fn as_ptr<P>(&self) -> *const P {
        self.addr as *const P
    }

    /// Return the address as a `usize`.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.addr
    }

    /// Return the address as a `u64`.
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.addr as u64
    }

    /// Check if the address is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.addr == 0
    }

    /// Align the address down to the nearest page boundary. If the address is
    /// already page aligned, then it is returned as is.
    #[must_use]
    pub const fn page_align_down(&self) -> Self {
        Self {
            addr: self.addr & !(mmu::PAGE_SIZE - 1),
            _phantom: self._phantom,
        }
    }

    /// Check if the address is page aligned.
    #[must_use]
    pub const fn is_page_aligned(&self) -> bool {
        self.addr % mmu::PAGE_SIZE == 0
    }
}

impl Virtual<User> {
    /// The minimum valid user virtual address, assuming a 39-bit virtual
    /// address space.
    pub const START: Self = Self {
        _phantom: PhantomData,
        addr: 0x0000_0000_0000_0000,
    };

    /// The maximum valid user virtual address, assuming a 39-bit virtual
    /// address space.
    pub const END: Self = Self {
        _phantom: PhantomData,
        addr: 0x0000_7FFF_FFFF_FFFF,
    };

    /// Create a new user virtual address.
    ///
    /// # Panics
    /// This function will panic if the address is not in the user
    /// address space (as defined by [`START`] and [`END`]).
    #[must_use]
    pub const fn new(addr: usize) -> Self {
        match Self::try_new(addr) {
            None => panic!("User virtual address out of bounds"),
            Some(v) => v,
        }
    }

    /// Attempt to create a new user virtual address. If the address is not
    /// in the user address space (as defined by [`START`] and [`END`]), then
    /// `None` is returned.
    #[must_use]
    pub const fn try_new(addr: usize) -> Option<Self> {
        if addr <= Self::END.addr {
            Some(Self {
                _phantom: PhantomData,
                addr,
            })
        } else {
            None
        }
    }
}

impl Virtual<Kernel> {
    /// The minimum valid kernel virtual address, assuming a 39-bit virtual
    /// address space.
    pub const START: Self = Self {
        _phantom: PhantomData,
        addr: 0xFFFF_8000_0000_0000,
    };

    /// The maximum valid kernel virtual address, assuming a 39-bit virtual
    /// address space.
    pub const END: Self = Self {
        _phantom: PhantomData,
        addr: 0xFFFF_FFFF_FFFF_FFFF,
    };

    /// Create a new kernel virtual address.
    ///
    /// # Panics
    /// This function will panic if the address is not in the kernel
    /// address space (as defined by [`START`] and [`END`]).
    #[must_use]
    pub const fn new(addr: usize) -> Self {
        match Self::try_new(addr) {
            None => panic!("Kernel virtual address out of bounds"),
            Some(v) => v,
        }
    }

    /// Attempt to create a new kernel virtual address. If the address is not
    /// in the kernel address space (as defined by [`START`] and [`END`]),
    /// then `None` is returned.
    #[must_use]
    pub const fn try_new(addr: usize) -> Option<Self> {
        if addr >= Self::START.addr {
            Some(Self {
                _phantom: PhantomData,
                addr,
            })
        } else {
            None
        }
    }

    /// Create a new kernel virtual address from a pointer.
    ///
    /// # Panics
    /// This function will panic if the address is not in the kernel
    /// address space (as defined by [`START`] and [`END`]).
    #[must_use]
    pub fn from_ptr<P>(ptr: *const P) -> Self {
        Self::new(ptr as usize)
    }

    /// Align the address up to the nearest page boundary. If the address is
    /// already page aligned, then it is returned as is.
    ///
    /// # Panics
    /// This function will panic if the resulting address cannot fit into an
    /// `u64` (the address is greater than [`MAX`]).
    #[must_use]
    pub const fn page_align_up(&self) -> Self {
        Self::new((self.addr + mmu::PAGE_SIZE - 1) & !(mmu::PAGE_SIZE - 1))
    }
}

impl<T: Type> From<Virtual<T>> for usize {
    fn from(addr: Virtual<T>) -> Self {
        addr.as_usize()
    }
}

impl<T: Type> From<Virtual<T>> for u64 {
    fn from(addr: Virtual<T>) -> Self {
        addr.as_u64()
    }
}

impl<T: Type> IsAligned for Virtual<T> {
    fn is_aligned(&self, align: usize) -> bool {
        (self.addr & (align - 1)) == 0
    }
}
