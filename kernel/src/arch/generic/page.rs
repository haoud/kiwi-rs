use crate::arch::addr::{AllMemory, Kernel, Physical, Virtual};

/// Translate a physical address to a virtual address. This is only possible if
/// the physical address is mapped into the kernel's virtual address space.
///
/// If the physical address is not mapped, this function returns `None`.
#[must_use]
pub fn translate(physical: Physical<AllMemory>) -> Option<Virtual<Kernel>> {
    crate::arch::target::page::translate(physical)
}
