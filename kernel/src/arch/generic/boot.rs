use crate::{arch, library::lock::spin::Spinlock};

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
/// allocation request.
#[must_use]
pub fn allocate(size: usize, align: usize) -> *mut u8 {
    let mut mmap = BOOT_MEMORY_MAP.lock();
    let entry = mmap
        .as_mut()
        .expect("Boot memory map not initialized or already reclaimed")
        .regions
        .iter_mut()
        .find(|entry| entry.kind == arch::mem::MemoryKind::Free && entry.len() >= size + align)
        .expect("Not enough free memory to satisfy the allocation request");

    let start = entry.start;
    let misalign = start % align;
    let padding = if misalign == 0 { 0 } else { align - misalign };
    let length = size + padding;

    entry.start += length;
    arch::page::translate(start + padding) as *mut u8
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
