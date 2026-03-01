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
    // SAFETY: The pause instruction is supported on all x86 CPUs and does not
    // have any side effects that could lead to memory unsafety. It just wastes
    // some CPU cycles in order to improve the performance of spinlock loops
    // (and I find funny the fact that wasting a few CPU cycles can actually
    // improve the performance of the system).
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
    }
}

/// Load the GDT with the provided address.
///
/// # Safety
/// The GDT must be properly formatted and must be located at the provided
/// address. The GDT must also remain valid and in the memory for the entire
/// lifetime of the kernel or until the GDT register is reloaded with another
/// address.
#[inline]
pub unsafe fn lgdt(address: usize) {
    core::arch::asm!(
        "lgdt [{}]",
        in(reg) address,
        options(nostack, preserves_flags)
    );
}

/// Load the IDT with the provided address.
///
/// # Safety
/// The IDT must be properly formatted and must be located at the provided
/// address. The IDT must also remain valid and in the memory for the entire
/// lifetime of the kernel or until the IDT register is reloaded with another
/// address.
#[inline]
pub unsafe fn lidt(address: usize) {
    core::arch::asm!(
        "lidt [{}]",
        in(reg) address,
        options(nostack, preserves_flags)
    );
}

/// Load the Task Register (TR) with the provided selector.
///
/// # Safety
/// The selector must point to a valid TSS entry in the GDT. The TSS entry must
/// be properly formatted and must point to a valid TSS structure in the
/// memory. The TSS structure must also remain valid and in the memory for the
/// entire lifetime of the kernel as well as the GDT structure that contains the
/// TSS entry.
pub unsafe fn ltr(selector: u16) {
    core::arch::asm!(
        "ltr ax",
        in("ax") selector,
        options(nomem, nostack, preserves_flags)
    );
}
