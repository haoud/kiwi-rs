/// The base address of the Higher Half Direct Map (HHDM). This is the virtual
/// address at which the entire physical memory is mapped in the kernel space.
/// This allows the kernel to access any physical memory directly using virtual
/// addresses, greatly simplifying memory management and access.
const HHDM_BASE: usize = 0xFFFF_8000_0000_0000;

/// Translates a physical address to its corresponding virtual address in the
/// Higher Half Direct Map (HHDM).
#[must_use]
pub fn translate(physical: usize) -> usize {
    HHDM_BASE + physical
}
