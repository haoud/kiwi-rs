use crate::mmu::{Physical, Virtual};

/// Translate a physical address to a virtual address, allowing the kernel to access
/// it. On RISC-V64, the translation is fixed and all physical addresses are mapped
/// inside the kernel's virtual address space.
#[must_use]
pub fn translate(_phys: Physical) -> Option<Virtual> {
    todo!("Use fixed translation for RISC-V64")
}
