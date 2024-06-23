use crate::arch::{
    self,
    mmu::{self, Align},
    target::addr::Physical,
};
use bitflags::bitflags;
use core::ops::{AddAssign, SubAssign};
use seqlock::Seqlock;

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

/// The number of reserved memory pages. These pages are not available for
/// allocation. Some of these pages are strictly reserved by the hardware
/// and cannot be used by the kernel (for example, the firemware data) while
/// others can be used by the kernel for specific purposes (for example, the
/// framebuffer or the memory mapped I/O regions).
static RESERVED_MEMORY_PAGES: spin::Mutex<usize> = spin::Mutex::new(0);

/// The number of kernel memory pages. These pages are used by the kernel for
/// its own data structures and are not available for allocation. This includes
/// static code and data, the kernel heap, the kernel stack, etc.
static KERNEL_MEMORY_PAGES: spin::Mutex<usize> = spin::Mutex::new(0);

/// The number of total memory pages. This is the total number of pages that
/// are available for allocation.
static TOTAL_MEMORY_PAGES: Seqlock<usize> = Seqlock::new(0);

/// The bitmap allocator is used to allocate and deallocate physical frames
/// using a bitmap. This allocator is very slow, but does not consume a lot
/// of memory (and is can be improved by using bit instead of bool).
static BITMAP: spin::Once<spin::Mutex<&mut [(bool, bool)]>> = spin::Once::new();

/// Initialize the physical memory manager
#[inline]
pub fn setup(mut memory: arch::memory::UsableMemory) {
    let frame_count = usize::from(memory.last_address()) / arch::mmu::PAGE_SIZE;
    let bitmap_size = frame_count * core::mem::size_of::<(bool, bool)>();

    log::info!("Initializing physical memory manager");
    log::debug!("Bitmap size: {} bytes", bitmap_size);

    // Allocate the bitmap
    let bitmap = unsafe {
        let base = memory
            .allocate_memory::<(bool, bool)>(bitmap_size, 16)
            .expect("Failed to allocate bitmap");

        let ptr = arch::mmu::translate_physical(base)
            .expect("Failed to translate bitmap physical address")
            .as_mut_ptr::<(bool, bool)>();

        core::slice::from_raw_parts_mut(ptr, frame_count)
    };

    *RESERVED_MEMORY_PAGES.lock() = memory.firmware_memory.page_count_up();
    *KERNEL_MEMORY_PAGES.lock() = memory.kernel_memory.page_count_up();
    TOTAL_MEMORY_PAGES.write(memory.total_memory.page_count_up());

    // Set all free frames to true
    memory
        .into_free_regions()
        .into_iter()
        .for_each(|memory_region| {
            let end = Physical::new(memory_region.start + memory_region.length)
                .page_align_up()
                .frame_idx();
            let start = Physical::new(memory_region.start)
                .page_align_up()
                .frame_idx();

            (start..end).for_each(|frame| {
                bitmap[frame] = (true, false);
            });
        });

    // Initialize the bitmap
    BITMAP.call_once(|| spin::Mutex::new(bitmap));
}

/// Allocate a frame. Returns `None` if no frame is available, or a frame if a
/// frame was successfully allocated.
#[must_use]
pub fn allocate_frame(flags: AllocationFlags) -> Option<Physical> {
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
) -> Option<Physical> {
    let kernel = flags.contains(AllocationFlags::KERNEL);
    let mut bitmap = BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock();

    // Find the first range of contiguous free frames
    let start = bitmap
        .windows(count)
        .position(|frames| frames.iter().all(|&(free, _)| free))?;

    // Mark the frames as used
    (start..start + count).for_each(|frame| {
        bitmap[frame] = (false, kernel);
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

    // Update the number of kernel memory pages
    if kernel {
        KERNEL_MEMORY_PAGES.lock().add_assign(count);
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
pub fn deallocate_frame(frame: Physical) {
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
pub fn deallocate_range(base: Physical, count: usize) {
    let start = base.frame_idx();
    let end = start + count;

    let mut bitmap = BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock();

    assert!(base.is_page_aligned());
    assert!(start + count >= start);
    assert!(start + count <= bitmap.len());

    let mut kernel = 0;
    (start..end).for_each(|frame| {
        assert!(!bitmap[frame].0, "Frame already deallocated");
        kernel += usize::from(bitmap[frame].1);
        bitmap[frame] = (true, false);
    });

    if kernel > 0 {
        KERNEL_MEMORY_PAGES.lock().sub_assign(kernel);
    }
}

/// Return the total number of memory pages in the system
#[must_use]
pub fn total_memory_pages() -> usize {
    TOTAL_MEMORY_PAGES.read()
}

/// Return the number of memory pages that are reserved by the hardware
/// or by the firmware and are not available for allocation.
#[must_use]
pub fn reserved_memory_pages() -> usize {
    *RESERVED_MEMORY_PAGES.lock()
}

/// Return the number of memory pages that are used by the kernel and are
/// not available for allocation.
#[must_use]
pub fn kernel_memory_pages() -> usize {
    *KERNEL_MEMORY_PAGES.lock()
}

/// Convert a frame index to a physical address
///
/// # Panics
/// Panics if the resulting physical address would be invalid (greater than
/// [`Physical::MAX`])
#[must_use]
const fn index2phys(index: usize) -> Physical {
    Physical::new(index * mmu::PAGE_SIZE)
}
