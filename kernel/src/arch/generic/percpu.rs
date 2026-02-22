use core::{mem::MaybeUninit, ops::Deref};

use crate::arch;

unsafe extern "C" {
    static __percpu_start: [u64; 0];
    static __percpu_end: [u64; 0];

    static __percpu_ctors_start: [u64; 0];
    static __percpu_ctors_end: [u64; 0];
}

/// Represents uninitialized storage for a per-CPU variable of type `T`. This
/// should only be used internally by the `#[per_cpu]` macro but is public to
/// allow the macro to create instances of it for the variables annotated with
/// `#[per_cpu]` anywhere in the kernel.
pub struct PerCpuStorage<T> {
    inner: MaybeUninit<T>,
}

/// SAFETY: `PerCpuStorage<T>` is safe to be shared between threads because its
/// role is only to reserve memory in the `.percpu` section. It is never read
/// or written and does not hold any meaningful data.
unsafe impl<T> Sync for PerCpuStorage<T> {}

impl<T> PerCpuStorage<T> {
    /// Creates a new `PerCpuStorage<T>` that contains uninitialized memory for
    /// a value of type `T`.
    ///
    /// # Safety
    /// The caller must ensure that the created `PerCpuStorage<T>` is properly
    /// placed in the `.percpu` section of the kernel.
    ///
    /// This function should only be used by the `#[per_cpu]` macro internally
    /// to create instances of `PerCpuStorage<T>` for the variables annotated
    /// with `#[per_cpu]`.
    #[must_use]
    pub const unsafe fn new() -> Self {
        Self {
            inner: MaybeUninit::uninit(),
        }
    }

    /// Returns the offset of the per-CPU variable in this `PerCpuStorage<T>`
    /// from the start of the per-CPU section.
    #[must_use]
    pub fn percpu_offset(&'static self) -> usize {
        // SAFETY: The pointer is a valid pointer to a location in the per-CPU
        // section of the kernel that contains a properly aligned value of type
        // `T` (as guaranteed by the `#[per_cpu]` macro that creates instances
        // of `PerCpuStorage<T>`).
        unsafe { get_offset(self.inner.as_ptr()) }
    }
}

/// Represents a per-CPU variable of type `T`. This struct provides methods to
/// safely access the per-CPU variable for the current CPU.
///
/// # Note
/// Do not use this struct directly, instead use the `#[per_cpu]` macro on a
/// static variable to create a per-CPU variable and access it through the
/// generated API.
pub struct PerCpu<T> {
    storage: *const PerCpuStorage<T>,
}

/// SAFETY: `PerCpu<T>` is safe to be shared between threads since each core
/// has its own instance of the per-CPU variable and therefore can only be
/// accessed by the thread running on that core. By using a `PerCpuGuard` to
/// access the variable, we also ensure that preemption and interruptions are
/// disabled while the variable is accessed, which prevents any potential
/// aliasing issues.
/// NOTE: Synchronous traps can still break aliasing rules in unexpected ways,
/// therefore the caller must ensure that no synchronous traps can occur while
/// accessing the variable. Failing to do so will lead to undefined behavior.
unsafe impl<T> Sync for PerCpu<T> {}

impl<T> PerCpu<T> {
    /// Creates a new `PerCpu<T>` that provides access to the per-CPU variable.
    ///
    /// # Safety
    /// This function should only be used by the `#[per_cpu]` macro internally
    /// to ensure that the following conditions are met:
    /// - The pointer provided to this function must be a valid pointer to a
    ///   `PerCpuStorage<T>` instance that is properly placed in the `.percpu`
    ///   section of the kernel.
    /// - The `PerCpuStorage<T>` instance pointed to by the provided pointer
    ///   must contain a properly aligned value of type `T`.
    #[must_use]
    pub const unsafe fn new(storage: *const PerCpuStorage<T>) -> Self {
        Self { storage }
    }

    /// Returns a guard that provides access to the per-CPU variable for the
    /// current CPU.
    #[must_use]
    pub fn local(&'static self) -> PerCpuGuard<'static, T> {
        // SAFETY: The pointer is not null and can be converted to a reference:
        // - The pointer is properly aligned for `T`
        // - The pointer points to a valid value of type `T`
        // - We don't violate any aliasing rules by creating a reference to it
        // We encapsulate the reference into a `PerCpuGuard` to ensure that
        // preemption is disabled while the reference is alive.
        PerCpuGuard::new(unsafe { self.local_unchecked().as_ref().unwrap_unchecked() })
    }

    /// Get a raw pointer to the per-CPU variable for the current CPU without
    /// any safety checks.
    ///
    /// # Safety
    /// The caller must ensure that the per-CPU storage has been properly
    /// initialized before calling this function. The caller must be careful
    /// to not break aliasing rules when using the returned pointer (be aware
    /// of preemption, interruptions and exceptions that can break aliasing
    /// rules in unexpected ways !).
    #[must_use]
    pub unsafe fn local_unchecked(&'static self) -> *const T {
        from_offset(get_offset(self.storage))
    }
}

pub struct PerCpuGuard<'a, T> {
    inner: &'a T,
}

impl<'a, T> PerCpuGuard<'a, T> {
    #[must_use]
    const fn new(inner: &'a T) -> Self {
        // TODO: Disable preemption & interrupts
        Self { inner }
    }
}

impl<T> Deref for PerCpuGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<T> Drop for PerCpuGuard<'_, T> {
    fn drop(&mut self) {
        // TODO: Reenable preemption and restore interrupts
    }
}

/// Initializes the per-CPU subsystem by allocating memory for the per-CPU
/// section and calling the architecture-specific setup function.
pub fn setup() {
    // Allocate per-cpu storage
    let percpu_start = core::ptr::addr_of!(__percpu_start).addr();
    let percpu_end = core::ptr::addr_of!(__percpu_end).addr();
    let percpu_size = percpu_end - percpu_start;
    let percpu = crate::arch::boot::allocate(percpu_size, 16);

    // Call the architecture-specific setup function
    // SAFETY: We provide a valid pointer to a memory region of the correct
    // size and alignment for the per-CPU section
    unsafe {
        arch::target::percpu::setup(percpu);
    }

    let percpu_ctor_start = (&raw const __percpu_ctors_start).cast::<fn()>();
    let percpu_ctor_end = (&raw const __percpu_ctors_end).cast::<fn()>();

    // SAFETY: All conditions required for `from_raw_parts` are satisfied (I'm
    // not going to list them all here, check the documentation for
    // `from_raw_parts` for details). Furthermore, we can guarantee that
    // `percpu_ctor_end` is greater than or equal to `percpu_ctor_start` (as
    // defined in the linker script) as required by `offset_from_unsigned`.
    let ctors = unsafe {
        core::slice::from_raw_parts(
            percpu_ctor_start,
            percpu_ctor_end.offset_from_unsigned(percpu_ctor_start),
        )
    };

    for constructor in ctors {
        constructor();
    }
}

/// Returns a pointer to a per-CPU variable at the given offset into the per-CPU
/// section.
///
/// # Safety
/// This function only computes a pointer and does not dereference it, so it is
/// not unsafe by itself and not marked as such.
///
/// However, If the offset is not in the per-CPU section or if the data at the
/// offset is not properly aligned for the type `T` or does not contain a valid
/// value of type `T`, then dereferencing the returned pointer is UB.
#[must_use]
pub fn from_offset<T>(offset: usize) -> *const T {
    crate::arch::target::percpu::from_offset(offset)
}

/// # Safety
/// The pointer provided to this function must be a valid pointer to a
/// location in the per-CPU section of the kernel (.percpu) that
/// contains a properly aligned value of type `T`.
#[must_use]
pub unsafe fn get_offset<T>(ptr: *const T) -> usize {
    ptr.addr() - core::ptr::addr_of!(__percpu_start).addr()
}
