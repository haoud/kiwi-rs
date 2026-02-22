use crate::arch::x86_64::msr;

/// A structure representing the per-CPU data for the `x86_64` architecture.
/// Kiwi reserves the first 64 bytes of the per-CPU data area for internal use.
/// We can use this space to retrieve the base address of the per-CPU data area
/// and to store the kernel and user stack pointers during a syscall.
pub struct PerCpuArchData {
    /// The base address of the per-CPU data.
    pub percpu_base: usize,

    /// The kernel stack pointer to use when handling a syscall.
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
