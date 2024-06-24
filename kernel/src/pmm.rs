use crate::arch::{
    self,
    mmu::{self, Align, PAGE_SIZE},
    target::addr::Physical,
};
use bitflags::bitflags;
use seqlock::Seqlock;

/// Informations about a frame.
#[derive(Debug)]
pub struct FrameInfo {
    flags: FrameFlags,
}

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

    /// Some frame flags to indicate some specificities about the frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FrameFlags: u8 {
        /// If set, the frame is free to be allocated
        const FREE = 1 << 0;

        /// If set, the frame is used by the kernel. It cannot be set if the
        /// `FREE` or `FIRMWARE` flags are set.
        const KERNEL = 1 << 1;

        /// If set, the frame is used by the firmware. It cannot be set if
        /// the `FREE` or `KERNEL` flags are set.
        const FIRMWARE = 1 << 2;
    }
}

/// The number of total memory pages. This is the total number of pages that
/// are available for allocation.
static TOTAL_MEMORY_PAGES: Seqlock<usize> = Seqlock::new(0);

/// The starting offset of the DRAM. This is useful for some architecture when
/// the RAM does not start at the address 0 and allow reduce the memory used by
/// the frame info array
static RAM_START: Seqlock<usize> = Seqlock::new(0);

/// The last address of RAM
static RAM_END: Seqlock<usize> = Seqlock::new(0);

/// The bitmap allocator is used to allocate and deallocate physical frames
/// using a bitmap. This allocator is very slow, but does not consume a lot
/// of memory and is "good enought" for now.
static BITMAP: spin::Once<spin::Mutex<&mut [FrameInfo]>> = spin::Once::new();

/// Initialize the physical memory manager
#[inline]
pub fn setup(mut memory: arch::memory::UsableMemory) {
    let frame_count = memory.ram_size().page_count_up();
    let bitmap_size = frame_count * core::mem::size_of::<FrameInfo>();

    log::info!("Initializing physical memory manager");
    log::debug!("Bitmap size: {} bytes", bitmap_size);

    // Allocate the bitmap by using a free memory region from
    // the memory map big enough for the bitmap
    let bitmap = unsafe {
        let base = memory
            .allocate_memory::<FrameInfo>(bitmap_size, 16)
            .expect("Failed to allocate bitmap");

        let ptr = arch::mmu::translate_physical(base)
            .expect("Failed to translate bitmap physical address")
            .as_mut_ptr::<FrameInfo>();

        // Initialize the bitmap before creating a slice (it would
        // be UB otherwise)
        for i in 0..frame_count {
            ptr.add(i).write(FrameInfo {
                flags: FrameFlags::KERNEL,
            })
        }

        // Create the slice
        core::slice::from_raw_parts_mut(ptr, frame_count)
    };

    TOTAL_MEMORY_PAGES.write(memory.total_memory.page_count_up());
    RAM_START.write(memory.ram_start);
    RAM_END.write(memory.ram_end);

    // Add the free flags to all available memory pages
    memory
        .into_free_regions()
        .into_iter()
        .for_each(|memory_region| {
            let start = phys2index(
                Physical::new(memory_region.start)
                    .page_align_up()
                    .as_usize(),
            );
            let end = phys2index(
                Physical::new(memory_region.end())
                    .page_align_up()
                    .as_usize(),
            );
            (start..end).for_each(|frame| {
                bitmap[frame].flags &= !FrameFlags::KERNEL;
                bitmap[frame].flags |= FrameFlags::FREE;
            });
        });

    // Reserve the memory used by the firmware (OpenSBI)
    // TODO: Make this architecture agnostic
    (0x80000000..0x80200000).for_each(|addr| {
        bitmap[phys2index(addr)].flags &= !FrameFlags::KERNEL;
        bitmap[phys2index(addr)].flags |= FrameFlags::FIRMWARE;
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
    let mut bitmap = BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock();

    // Find the first range of contiguous free frames
    let start = bitmap.windows(count).position(|frames| {
        frames
            .iter()
            .all(|info| info.flags.contains(FrameFlags::FREE))
    })?;

    // Mark the frames as used and add the kernel flags to
    // frames if requested
    for frame in start..start + count {
        bitmap[frame].flags.remove(FrameFlags::FREE);
        if flags.contains(AllocationFlags::KERNEL) {
            bitmap[frame].flags |= FrameFlags::KERNEL;
        }
    }

    // Zero the frames if requested
    if flags.contains(AllocationFlags::ZEROED) {
        let ptr = arch::mmu::translate_physical(index2phys(start))
            .expect("Failed to translate physical address")
            .as_mut_ptr::<u8>();

        // SAFETY: Zeroing the frame is safe since it isn't used
        // by anything else and will not cause undefined behavior
        unsafe {
            core::ptr::write_bytes(ptr, 0, PAGE_SIZE * count);
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
pub fn deallocate_frame(frame: Physical) {
    deallocate_range(frame, 1);
}

/// Deallocate a contiguous range of frames starting at the given base address
///
/// # Panics
/// Panics if at least one of the following conditions is met:
/// - The base address is not page-aligned
/// - The range is not allocated
/// - The range is outside of the bitmap
pub fn deallocate_range(base: Physical, count: usize) {
    let start = phys2index(usize::from(base));
    let end = start + count;

    let mut bitmap = BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock();

    assert!(base.is_page_aligned());
    assert!(start + count >= start);
    assert!(start + count <= bitmap.len());

    (start..end).for_each(|frame| {
        assert!(!bitmap[frame].flags.contains(FrameFlags::FREE));
        bitmap[frame].flags.remove(FrameFlags::KERNEL);
        bitmap[frame].flags.insert(FrameFlags::FREE);
    });
}

/// Return the total number of memory pages in the system
#[must_use]
pub fn total_memory_pages() -> usize {
    TOTAL_MEMORY_PAGES.read()
}

/// Return the number of memory pages that are used by the kernel and are
/// not available for allocation, including reserved memory by the firmware
/// or the hardware
#[must_use]
pub fn kernel_memory_pages() -> usize {
    BITMAP
        .get()
        .expect("Physical memory bitmap not initialized")
        .lock()
        .iter()
        .filter(|frame| frame.flags.contains(FrameFlags::KERNEL))
        .count()
}

/// Convert a frame index to a physical address
///
/// # Panics
/// Panics if the resulting physical address would be invalid (greater than
/// [`Physical::MAX`])
#[must_use]
fn index2phys(index: usize) -> Physical {
    Physical::new(RAM_START.read() + index * mmu::PAGE_SIZE)
}

/// Convert a physical address to an index into the bitmap
///
/// # Panics
/// Panics if the physical addresse in outside of the bitmap
fn phys2index(addr: usize) -> usize {
    assert!(addr >= RAM_START.read());
    assert!(addr <= RAM_END.read());
    (addr - RAM_START.read()) / PAGE_SIZE
}
