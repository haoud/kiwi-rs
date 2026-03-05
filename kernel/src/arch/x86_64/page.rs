use crate::arch::addr::{AllMemory, Kernel, Physical, Virtual};

/// The base address of the Higher Half Direct Map (HHDM). This is the virtual
/// address at which the entire physical memory is mapped in the kernel space.
/// This allows the kernel to access any physical memory directly using virtual
/// addresses, greatly simplifying memory management and access.
const HHDM_BASE: Virtual<Kernel> = Virtual::<Kernel>::new(0xFFFF_8000_0000_0000);

/// The maximum address of the Higher Half Direct Map (HHDM). This is an
/// arbitrary limit set to avoid the HHDM from taking up the entire virtual
/// address space. To access physical memory beyond this limit, the kernel
/// would need to map it explicitly using page tables.
const HHDM_MAX_ADDRESS: Virtual<Kernel> = Virtual::<Kernel>::new(0xFFFF_9000_0000_0000);

/// Translates a physical address to its corresponding virtual address in the
/// Higher Half Direct Map (HHDM). If the physical address is not mapped in the
/// HHDM or if the translation would overflow, this function returns `None`.
#[must_use]
pub fn translate(physical: Physical<AllMemory>) -> Option<Virtual<Kernel>> {
    Some(Virtual::<Kernel>::new(
        HHDM_BASE.as_usize().checked_add(physical.as_usize())?,
    ))
}

/// Translates a virtual address in the Higher Half Direct Map (HHDM) back to
/// its corresponding physical address. If the virtual address is not within
/// the HHDM, this function returns `None`.
#[must_use]
pub fn from_hhdm(address: Virtual<Kernel>) -> Option<Physical<AllMemory>> {
    if address >= HHDM_BASE && address < HHDM_MAX_ADDRESS {
        Some(Physical::<AllMemory>::new(
            usize::from(address) - usize::from(HHDM_BASE),
        ))
    } else {
        None
    }
}
