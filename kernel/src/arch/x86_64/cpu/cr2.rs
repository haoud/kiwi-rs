use crate::arch::addr::VirtualAddress;

/// Read the value of the CR2 register.
///
/// The CR2 register contains the linear address that caused a page fault. This
/// function is typically used in page fault handlers to determine the address
/// that caused the fault and to take appropriate action, such as loading the
/// required page into memory or terminating the offending process.
#[must_use]
pub fn read() -> VirtualAddress {
    let cr2: usize;

    // SAFETY: Reading the CR2 register should be safe and does not cause any
    // memory unsafety or undefined behavior.
    unsafe {
        core::arch::asm!(
            "mov {}, cr2",
            out(reg) cr2,
            options(nomem, nostack, preserves_flags)
        );
    }
    VirtualAddress::new(cr2)
}
