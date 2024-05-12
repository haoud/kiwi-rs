use crate::pmm;

/// The global heap allocator. This allocator is used to allocate
/// memory on the kernel heap. However, the kernel heap should only
/// used to allocate relatively small chunks of memory. Large
/// allocations should be done using the virtual memory allocator
///(not yet implemented).
#[global_allocator]
static ALLOCATOR: talc::Talck<spin::Mutex<()>, OomHandler> =
    talc::Talck::new(talc::Talc::new(OomHandler {}));

/// The global OOM handler when the kernel heap is exhausted. This
/// handler will allocate enough physical memory to satisfy the
/// allocation request. If the system is truly out of memory, the
/// kernel will panic.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct OomHandler {}

impl OomHandler {
    /// The size of the allocation that will be attempted when
    /// handling an OOM. The value should be not too small to
    /// avoid invoking the OOM handler too often, but not too
    /// large to avoid allocating too much contiguous physical
    /// memory that could lead to an fatal OOM, even if there
    /// is enough memory available (but not contiguous).
    const ALLOCATION_SIZE: usize = 128 * 1024;
}

impl talc::OomHandler for OomHandler {
    fn handle_oom(
        talc: &mut talc::Talc<Self>,
        layout: core::alloc::Layout,
    ) -> Result<(), ()> {
        // The heap should not be used to allocate large chunks of
        // memory. Since kiwi is designed to be a microkernel, this
        // should never happen.
        if layout.size() > Self::ALLOCATION_SIZE {
            log::error!(
                "Allocation request too large: {} bytes",
                layout.size()
            );
            return Err(());
        }

        log::debug!(
            "Kernel heap exhausted, attempting to allocate {} more bytes",
            Self::ALLOCATION_SIZE
        );

        // Allocate 128KiB of contiguous physical memory
        let count = Self::ALLOCATION_SIZE >> arch::mmu::PAGE_SHIFT;
        let base = pmm::allocate_range(count, pmm::AllocationFlags::KERNEL)
            .ok_or(())?;

        // Convert the physical address to a virtual address and
        // compute the start and end pointers of the allocation.
        let memory = arch::mmu::translate_physical(base).ok_or(())?;
        let start = memory.as_mut_ptr::<u8>();
        let end = memory
            .as_mut_ptr::<u8>()
            .wrapping_byte_add(Self::ALLOCATION_SIZE);

        // SAFETY: The given span is valid, does not overlapp with any
        // other span, is not in use anywhere else in the system and is
        // valid for reads and writes.
        unsafe { talc.claim(talc::Span::new(start, end)).map(|_| ()) }
    }
}

/// Setup the global kernel heap allocator.
#[inline]
pub fn setup() {
    log::info!("Setting up the kernel heap allocator");
    // The heap will be initialized by the global allocator when the
    // first allocation will be requested.
}
