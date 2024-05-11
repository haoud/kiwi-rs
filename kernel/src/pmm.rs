use bitflags::bitflags;

bitflags! {
    /// Allocation flags that can be used to customize the behavior of
    /// the physical memory allocator or to provide additional information
    /// about the allocated frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AllocationFlags: u8 {
        /// The frame will be used by the kernel. This does not have any
        /// effect on the allocation itself, but can be used to track which
        /// frames are used by the kernel.
        const KERNEL = 1 << 0;

        /// The frame will be zeroed before it is returned to the caller.
        const ZEROED = 1 << 1;
    }
}

/// The bitmap allocator is used to allocate and deallocate physical frames
/// using a bitmap. This allocator is very slow, but does not consume a lot
/// of memory (and is can be improved by using bit instead of bool).
static BITMAP: spin::Once<spin::Mutex<&mut [bool]>> = spin::Once::new();

/// Initialize the physical memory manager
#[inline]
pub fn setup(mut memory: arch::memory::UsableMemory) {
    let frame_count = usize::from(memory.last_address()) / arch::mmu::PAGE_SIZE;
    let bitmap_size = frame_count * core::mem::size_of::<bool>();

    // Allocate the bitmap
    let bitmap = unsafe {
        let base = memory
            .allocate_memory::<bool>(bitmap_size, 16)
            .expect("Failed to allocate bitmap");

        let ptr = arch::mmu::translate_physical(base)
            .expect("Failed to translate bitmap physical address")
            .as_mut_ptr::<bool>();

        core::slice::from_raw_parts_mut(ptr, frame_count)
    };

    // Set all free frames to true
    memory
        .into_free_regions()
        .into_iter()
        .for_each(|memory_region| {
            let end = arch::mmu::Physical::new(
                memory_region.start + memory_region.length,
            )
            .page_align_up()
            .frame_idx();
            let start = arch::mmu::Physical::new(memory_region.start)
                .page_align_up()
                .frame_idx();

            (start..end).for_each(|frame| {
                bitmap[frame] = true;
            });
        });

    BITMAP.call_once(|| spin::Mutex::new(bitmap));
}

/// Allocate a frame. Returns `None` if no frame is available, or a frame if a
/// frame was successfully allocated.
#[must_use]
pub fn allocate_frame(flags: AllocationFlags) -> Option<arch::mmu::Physical> {
    allocate_range(1, flags)
}

/// Allocate a contiguous range of frames. Returns `None` if no contiguous
/// range of frames is available. This does not mean that there are no free
/// frames, but simply that there are no contiguous free frames (e.g. due to
/// fragmentation).
#[must_use]
pub fn allocate_range(
    count: usize,
    flags: AllocationFlags,
) -> Option<arch::mmu::Physical> {
    let mut bitmap = BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock();

    // Find the first range of contiguous free frames
    let start = bitmap
        .windows(count)
        .position(|frames| frames.iter().all(|&b| b))?;

    // Mark the frames as used
    (start..start + count).for_each(|frame| {
        bitmap[frame] = false;
    });

    // Zero the frames if requested
    if flags.contains(AllocationFlags::ZEROED) {
        let ptr = arch::mmu::translate_physical(index2phys(start))
            .expect("Failed to translate physical address")
            .as_mut_ptr::<u8>();
        unsafe {
            core::ptr::write_bytes(ptr, 0, arch::mmu::PAGE_SIZE * count);
        }
    }

    Some(index2phys(start))
}

/// Deallocate a frame
///
/// # Panics
/// Panics if at least one of the following conditions is met:
/// - The frame is not page-aligned
/// - The frame is not allocated (double free ?)
/// - The frame is outside of the bitmap (kernel bug ?)
pub fn deallocate_frame(frame: arch::mmu::Physical) {
    deallocate_range(frame, 1);
}

/// Deallocate a contiguous range of frames starting at the given base address
///
/// # Panics
/// Panics if at least one of the following conditions is met:
/// - The base address is not page-aligned
/// - The range is not allocated (double free ?)
/// - The range is outside of the bitmap (kernel bug ?)
/// - The range wraps around the end of the bitmap (kernel bug ?)
pub fn deallocate_range(base: arch::mmu::Physical, count: usize) {
    let start = base.frame_idx();
    let end = start + count;

    let mut bitmap = BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock();

    assert!(base.is_page_aligned());
    assert!(start + count >= start);
    assert!(start + count <= bitmap.len());

    (start..end).for_each(|frame| {
        assert!(!bitmap[frame], "Frame already deallocated");
        bitmap[frame] = true;
    });
}

/// Convert a frame index to a physical address
#[must_use]
const fn index2phys(index: usize) -> arch::mmu::Physical {
    arch::mmu::Physical::new(index * arch::mmu::PAGE_SIZE)
}
