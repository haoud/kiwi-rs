/// A physical address. This is used to provide a type-safe way to represent
/// physical addresses, and its implementation varies depending on the architecture.
///
/// A physical address represents a location in the physical memory of the system.
/// Each physical address is unique, but cannot be used to access the memory and must
/// be mapped to a virtual address before being used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Physical(pub(crate) usize);

/// A virtual address. This is used to provide a type-safe way to represent
/// virtual addresses, and its implementation varies depending on the architecture.
///
/// A virtual address represents a location in the virtual memory of the system.
/// Different virtual addresses can point to the same physical address, and the
/// translation between virtual and physical addresses is done by the Memory Management
/// Unit (MMU) of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Virtual(pub(crate) usize);

/// Translate a physical address to a virtual address. If the translation cannot be
/// done, this function will return `None`. This often happens when the physical
/// address cannot be mapped to a virtual address because the virtual address is
/// too small. This should only happen on 32-bit systems with more than 4 GiB of RAM,
/// which is not common.
#[must_use]
pub fn translate(phys: Physical) -> Option<Virtual> {
    crate::target::mmu::translate(phys)
}
