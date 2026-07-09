use crate::arch::x86_64::msr;

/// A structure representing the per-CPU data for the `x86_64` architecture.
/// Kiwi reserves the first 64 bytes of the per-CPU data area for internal use.
/// We can use this space to retrieve the base address of the per-CPU data area
/// and to store the kernel and user stack pointers during a syscall.
pub struct PerCpuArchData {
    /// The base address of the per-CPU data.
    pub percpu_base: usize,

    /// The kernel stack pointer for the per-cpu event loop. If zero, this
    /// means that we already inside a trap handler and we should not switch
    /// to the kernel stack again.
    pub kstack: usize,

    /// The saved user stack pointer to restore when returning from a syscall.
    pub ustack: usize,
}

/// Initializes the per-CPU data area for the current CPU by setting up the GS
/// segment register to point to the provided `percpu` data area.
///
/// # Safety
/// The caller must ensure that the `percpu` pointer points to a valid per-CPU
/// data area, accessible in both read and write mode, is properly aligned and
/// is large enough to hold the per-CPU data structure. Additionally, the caller
/// must ensure that this function is called only once per CPU.
/// If you are not the [``crate::arch::percpu::setup``] function, you should not
/// use this function at all, as it is intended for internal use only.
pub unsafe fn setup(percpu: *mut u8) {
    let percpu_start = percpu.addr() as u64;
    let arch_percpu = PerCpuArchData {
        percpu_base: percpu.addr(),
        kstack: 0,
        ustack: 0,
    };

    msr::write(msr::Register::KERNEL_GS_BASE, percpu_start);
    msr::write(msr::Register::GS_BASE, percpu_start);

    // SAFETY: The pointer is valid, properly aligned, and is big enough
    // to hold the architecture-specific per-CPU data structure.
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        core::ptr::write(percpu.cast::<PerCpuArchData>(), arch_percpu);
    }
}

/// Sets the kernel stack pointer for the current CPU by writing the provided
/// `kstack` value to the GS segment register at offset 0x08.
///
/// Remember that the kernel stack pointer must be aligned at least to 16
/// bytes, and that the kernel stack grows downwards, so the `kstack` pointer
/// should point to the top of the allocated memory area for the kernel stack
///
/// # Safety
/// The caller must ensure that the `kstack` pointer points to a valid kernel
/// stack, accessible in both read and write mode, is properly aligned and is
/// large enough to hold the kernel stack. Additionally, the caller must ensure
/// that the stack pointer stays valid until another kernel stack is set, and
/// that this function is called after the per-CPU data area has been
/// initialized with [`setup`].
pub unsafe fn set_kstack(kstack: usize) {
    core::arch::asm!(
        "mov gs:0x08, {}",
        in(reg) kstack,
        options(nostack, preserves_flags)
    );
}

/// Sets the user stack pointer for the current CPU by writing the provided
/// `ustack` value to the GS segment register at offset 0x10.
///
/// Remember that the user stack pointer must be aligned at least to 16
/// bytes, and that the user stack grows downwards, so the `ustack` pointer
/// should point to the top of the allocated memory area for the user stack.
///
/// # Safety
/// The caller must ensure that the `ustack` pointer points to a valid user
/// stack, accessible in both read and write mode, is properly aligned and is
/// large enough to hold the user stack. Additionally, the caller must ensure
/// that the stack pointer stay valid until another user stack is set, and
/// that this function is called after the per-CPU data area has been
/// initialized with [`setup`].
pub unsafe fn set_ustack(ustack: usize) {
    core::arch::asm!(
        "mov gs:0x10, {}",
        in(reg) ustack,
        options(nostack, preserves_flags)
    );
}

/// See [`crate::arch::percpu::from_offset`] for documentation.
#[must_use]
pub fn from_offset<T>(offset: usize) -> *const T {
    let percpu: usize;

    // The GS segment register points to the base of the per-CPU data area.
    // Since we cannot read the value of GS directly (we could read it using
    // MSRs, but that would be too slow), we can read the value of the first
    // 8 bytes of the per-CPU data area, which contains the value of GS itself.
    //
    // SAFETY: Reading the first 8 bytes of the per-CPU data area after it has
    // been initialized is safe, as it will always contain the value of GS.
    unsafe {
        core::arch::asm!(
            "mov {}, gs:0",
            out(reg) percpu,
            options(nostack, readonly, preserves_flags)
        );
    }
    (percpu + offset) as *const T
}
