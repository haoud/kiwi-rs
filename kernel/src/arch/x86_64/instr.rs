/// Write an 8 bit value from a port.
///
/// # Safety
/// This function is unsafe because writing to a port can have side effects,
/// including causing the hardware to do something unexpected and possibly
/// violating memory safety.
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

/// Write an 16 bit value from a port.
///
/// # Safety
/// This function is unsafe because writing to a port can have side effects,
/// including causing the hardware to do something unexpected and possibly
/// violating memory safety.
#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack, preserves_flags)
    );
}

/// Write an 32 bit value from a port.
///
/// # Safety
/// This function is unsafe because writing to a port can have side effects,
/// including causing the hardware to do something unexpected and possibly
/// violating memory safety.
#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    core::arch::asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, nostack, preserves_flags)
    );
}

/// Read an 8 bit value from a port.
///
/// # Safety
/// This function is unsafe because reading from a port can have side effects,
/// including causing the hardware to do something unexpected and possibly
/// violating memory safety.
#[inline]
#[must_use]
pub unsafe fn inb(port: u16) -> u8 {
    let mut value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}

/// Read an 16 bit value from a port.
///
/// # Safety
/// This function is unsafe because reading from a port can have side effects,
/// including causing the hardware to do something unexpected and possibly
/// violating memory safety.
#[inline]
#[must_use]
pub unsafe fn inw(port: u16) -> u16 {
    let mut value: u16;
    core::arch::asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}

/// Read an 32 bit value from a port.
///
/// # Safety
/// This function is unsafe because reading from a port can have side effects,
/// including causing the hardware to do something unexpected and possibly
/// violating memory safety.
#[inline]
#[must_use]
pub unsafe fn inl(port: u16) -> u32 {
    let mut value: u32;
    core::arch::asm!(
        "in eax, dx",
        in("dx") port,
        out("eax") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}

/// Disable interrupts on the current CPU core.
///
/// # Safety
/// This function is unsafe because disabling interrupts can have side effects
/// and can freeze the computer if not used properly.
#[inline]
pub unsafe fn cli() {
    core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
}

/// Enable interrupts on the current CPU core.
///
/// # Safety
/// This function is unsafe because enabling interrupts can have side effects
/// and can lead to a triple fault or memory unsafety if the interrupts are
/// not properly handled.
#[inline]
pub unsafe fn sti() {
    core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
}

/// Halt the CPU until the next interrupt arrives.
///
/// # Safety
/// This function is unsafe because halting the CPU can have side effects,
/// especially if the interrupts are not enabled (hang the CPU forever).
#[inline]
pub unsafe fn hlt() {
    core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
}

/// Improve the CPU performance of spinlock loops. The processor uses this hint
/// to avoid the memory order violation that can occur when exiting a spinlock
/// loop. This instruction is a no-op on older CPUs and pause the CPU during a
/// very short time on newer CPUs. The duration of the pause is not specified
///  and can vary between CPU models.
#[inline]
pub fn pause() {
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
    }
}
