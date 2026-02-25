use limine::memory_map::EntryType;
use usize_cast::IntoUsize;

use crate::arch::{
    self,
    addr::{AllMemory, Physical},
};

/// Request the bootloader to provide a memory map.
static LIMINE_MEMMAP_REQUEST: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

/// Convert a `limine::memory_map::EntryType` to an `arch::mem::MemoryKind`.
impl From<limine::memory_map::EntryType> for arch::mem::MemoryKind {
    fn from(value: limine::memory_map::EntryType) -> Self {
        match value {
            EntryType::BOOTLOADER_RECLAIMABLE | EntryType::EXECUTABLE_AND_MODULES => Self::Kernel,
            EntryType::ACPI_NVS
            | EntryType::ACPI_RECLAIMABLE
            | EntryType::FRAMEBUFFER
            | EntryType::RESERVED => Self::Reserved,
            EntryType::USABLE => Self::Free,
            _ => Self::Poisoned,
        }
    }
}

/// Setup the boot memory map by requesting it from the bootloader and
/// converting it into the kernel's internal representation. This allows
/// the kernel to allocate memory before it has fully initialized its own
/// memory management system.
///
/// # Panics
/// Panics if the bootloader did not provide a memory map, or provided an
/// invalid memory map (e.g. with overlapping regions, or regions that exceed
/// the maximum physical address supported by the kernel).
pub fn setup() {
    let response = LIMINE_MEMMAP_REQUEST
        .get_response()
        .expect("No memory map provided by the bootloader");

    // Trust the bootloader to provide a valid memory map. If the bootloader is
    // malicious or buggy, this could lead to undefined behavior, but we have
    // no choice at this point in the boot process.
    let mut memmap = arch::generic::mem::MemoryMap::empty();
    for entry in response.entries() {
        let start_addr = entry.base.into_usize();
        let end_addr = start_addr
            .checked_add(entry.length.into_usize())
            .expect("Memory map entry exceeds physical address space bounds");
        let start = Physical::<AllMemory>::new(start_addr);
        let end = Physical::<AllMemory>::new(end_addr);
        let kind = arch::mem::MemoryKind::from(entry.entry_type);
        memmap.regions.push(arch::mem::Region { start, end, kind });
    }

    arch::boot::BOOT_MEMORY_MAP.lock().replace(memmap);
}
