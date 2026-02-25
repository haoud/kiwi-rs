use crate::{
    arch::{self, addr::PAGE_SIZE},
    library::lock::spin::Spinlock,
};

/// The boot memory map. If the boot memory map is `None`, it means that the
/// boot subsystem has not been initialized yet, or that the memory has already
/// been reclaimed by the kernel.
pub static BOOT_MEMORY_MAP: Spinlock<Option<arch::mem::MemoryMap>> = Spinlock::new(None);

/// Allocate a block of memory from the boot allocator. This is a simple bump
/// allocator that uses the memory map provided by the bootloader to find
/// free memory regions. This function is not optimized for performance since
/// the boot process happens only once and is not performance-critical.
///
/// # Panics
/// Panics if the boot memory map has not been initialized, if the boot memory
/// has been reclaimed or if there is not enough free memory to satisfy the
/// allocation request. Also panics if the requested alignment is not a power
/// of two or greater than a page size, or if the requested size is zero.
#[must_use]
pub fn allocate(size: usize, align: usize) -> *mut u8 {
    assert!(align.is_power_of_two(), "Alignment must be a power of two");
    assert!(align <= PAGE_SIZE, "Alignment must be at most a page size");
    assert!(size > 0, "Size must be greater than zero");

    let mut mmap = BOOT_MEMORY_MAP.lock();
    let entry = mmap
        .as_mut()
        .expect("Boot memory map not initialized or already reclaimed")
        .regions
        .iter_mut()
        .filter(|entry| entry.kind == arch::mem::MemoryKind::Free)
        .find(|entry| {
            let aligned_start = entry.start.align_up(align);
            entry.end >= aligned_start + size
        })
        .expect("Not enough free memory to satisfy the allocation request");

    let start = entry.start.align_up(align);
    entry.start = start + size;
    arch::page::translate(start)
        .expect("Failed to translate boot physical memory address to kernel memory")
        .as_mut_ptr::<u8>()
}

/// Allocate a block of memory from the boot allocator and zero it out. This is
/// a simple wrapper around [`allocate`] that calls `core::ptr::write_bytes` to
/// fill the allocated memory with zeros. See the documentation for [`allocate`]
/// for more details.
///
/// # Panics
/// See [`allocate`] for details on when this function can panic.
#[must_use]
pub fn allocate_zeroed(size: usize, align: usize) -> *mut u8 {
    let ptr = allocate(size, align);
    // SAFETY: The allocated memory is guaranteed to be valid for writes
    // and is not aliased, so writing zeros to it is safe.
    unsafe { core::ptr::write_bytes(ptr, 0, size) };
    ptr
}

/// Reclaim the memory used by the boot allocator. This should be called once
/// the kernel has finished basic initialization and is ready to take control
/// of the system's memory. After calling this function, trying to allocate
/// memory using the boot allocator will result in a panic.
///
/// # Panics
/// Panics if the boot memory map has not been initialized, or if the memory
/// was already reclaimed by the kernel.
#[must_use]
pub fn reclaim_memory() -> arch::mem::MemoryMap {
    // TODO: Align all free regions to page boundaries
    BOOT_MEMORY_MAP
        .lock()
        .take()
        .expect("Boot memory map not initialized or already reclaimed")
}
