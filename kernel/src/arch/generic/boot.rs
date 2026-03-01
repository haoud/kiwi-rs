use macros::init;

use crate::{
    arch::{
        self,
        addr::{AllMemory, PAGE_SIZE, Physical},
    },
    library::lock::spin::Spinlock,
};

/// The boot memory map. If the boot memory map is `None`, it means that the
/// boot subsystem has not been initialized yet, or that the memory has already
/// been reclaimed by the kernel.
static BOOT_MEMORY_MAP: Spinlock<Option<arch::mem::MemoryMap>> = Spinlock::new(None);

/// Setup the boot memory allocator by acquiring the memory map from the
/// bootloader and using it to allocate memory for the kernel during the early
/// stages of the boot process.
///
/// # Safety
/// This function must only be called once during the boot process, and must be
/// called before any other function in the `arch::boot` module is called.
#[init]
pub unsafe fn setup() {
    BOOT_MEMORY_MAP.lock().replace(arch::target::boot::setup());
}

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

    let mut mmap_lock = BOOT_MEMORY_MAP.lock();
    let mmap = mmap_lock
        .as_mut()
        .expect("Boot memory map not initialized or already reclaimed");
    let block_start;
    let block_end;

    // Find a free memory region that can accommodate the requested block of
    // memory with the required alignment, and update the region to reflect the
    // allocated block. This is a simple first-fit algorithm that iterates over
    // the free regions in the memory map and finds the first one that can
    // accommodate the requested block and updates `block_start` and `block_end`
    // to the start and end addresses of the allocated block.
    let allocated = {
        let free_entry = mmap
            .regions
            .iter_mut()
            .filter(|entry| entry.kind == arch::mem::MemoryKind::Free)
            .find(|entry| {
                let aligned_start = entry.start.align_up(align);
                entry.end >= aligned_start + size
            })
            .expect("Not enough free memory to satisfy the allocation request");

        let start = free_entry.start.align_up(align);
        block_start = free_entry.start;
        block_end = start + size;

        free_entry.start = block_end;
        arch::page::translate(start)
            .expect("Failed to translate boot physical memory address to kernel memory")
            .as_mut_ptr::<u8>()
    };

    // Find a used memory region that is adjacent to the free region to not lose
    // track of it in the memory map.
    let used_entry = mmap
        .regions
        .iter_mut()
        .filter(|entry| entry.kind == arch::mem::MemoryKind::Kernel)
        .find(|entry| entry.end == block_start);

    // If there is no such region, we add a new entry to the memory map to
    // represent the allocated block as a used region, otherwise we just
    // extend the existing region to cover the allocated block.
    if let Some(entry) = used_entry {
        entry.end = block_end;
    } else {
        mmap.regions.push(arch::mem::Region {
            start: block_start,
            end: block_end,
            kind: arch::mem::MemoryKind::Kernel,
        });
    }

    allocated
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

/// Allocate a block of memory from the boot allocator and fill it with the
/// given value, returning a mutable slice to the allocated memory with the
/// same lifetime as the kernel (i.e. `'static`).
///
/// # Panics
/// Panics if the boot memory map has not been initialized, if the boot memory
/// has been reclaimed, if there is not enough free memory to satisfy the
/// allocation request or if the requested size is zero.
#[must_use]
pub fn allocate_slice<T: Copy>(count: usize, obj: T) -> &'static mut [T] {
    let size = count * core::mem::size_of::<T>();
    let align = core::mem::align_of::<T>();
    let data = allocate(size, align).cast::<T>();

    for i in 0..count {
        // SAFETY: The allocated memory is guaranteed to be valid for writes
        // and is not aliased, so writing the data to it is safe.
        unsafe { data.add(i).write(obj) };
    }

    // SAFETY: The allocated memory is guaranteed to be non-null, properly
    // aligned, valid for reads and writes, not aliased... so returning a
    // reference to it is safe.
    unsafe { core::slice::from_raw_parts_mut(data, count) }
}

/// Allocate a block of memory from the boot allocator and fill it with the
/// values produced by the given function, returning a mutable slice to the
/// allocated memory with the same lifetime as the kernel (i.e. `'static`).
///
/// # Panics
/// Panics if the boot memory map has not been initialized, if the boot memory
/// has been reclaimed, if there is not enough free memory to satisfy the
/// allocation request or if the requested size is zero.
#[must_use]
pub fn allocate_slice_from_fn<T, F: Fn() -> T>(count: usize, f: F) -> &'static mut [T] {
    let size = count * core::mem::size_of::<T>();
    let align = core::mem::align_of::<T>();
    let data = allocate(size, align).cast::<T>();

    for i in 0..count {
        // SAFETY: The allocated memory is guaranteed to be valid for writes
        // and is not aliased, so writing the data to it is safe.
        unsafe { data.add(i).write(f()) };
    }

    // SAFETY: The allocated memory is guaranteed to be non-null, properly
    // aligned, valid for reads and writes, not aliased... so returning a
    // reference to it is safe.
    unsafe { core::slice::from_raw_parts_mut(data, count) }
}

/// Find the last regular physical address in the system. This is the highest
/// physical address that is not reserved for special purposes (e.g. MMIO, APIC
/// etc.) and can be used for normal memory allocations. This is useful for
/// setting up the page tables and the physical memory manager, since we need to
/// know the maximum physical address that we can use for memory allocations.
///
/// # Panics
/// Panics if the boot memory map has not been initialized or if the boot memory
/// was already reclaimed.
#[must_use]
pub fn last_regular_address() -> Physical<AllMemory> {
    BOOT_MEMORY_MAP
        .lock()
        .as_ref()
        .expect("Boot memory map not initialized or already reclaimed")
        .last_regular_address
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
