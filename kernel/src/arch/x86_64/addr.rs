use crate::arch::addr::{AllMemory, Kernel, PhysicalSpace, User, VirtualSpace};

/// The minimum page size for the architecture.
pub const PAGE_SIZE: usize = 4096;

/// The kernel's virtual address space
impl VirtualSpace for Kernel {
    const MIN: usize = 0xFFFF_8000_0000_0000;
    const MAX: usize = 0xFFFF_FFFF_FFFF_FFFF;
}

/// The user-space virtual address space
impl VirtualSpace for User {
    const MIN: usize = 0x0000_0000_0000_0000;
    const MAX: usize = 0x0000_7FFF_FFFF_FFFF;
}

/// The whole physical address space
impl PhysicalSpace for AllMemory {
    const MIN: usize = 0x0000_0000_0000_0000;

    const MAX: usize = 0x000F_FFFF_FFFF_FFFF;
}
