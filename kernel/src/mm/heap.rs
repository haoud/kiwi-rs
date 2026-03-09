use linked_list_allocator::LockedHeap;
use macros::init;

use crate::{arch, mm::buddy};

/// The global allocator for the kernel.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// The heap has a fixed size of 16 MiB, which is enough for testing purposes
/// until we implement a better memory allocator (slub).
const HEAP_ORDER: buddy::Order = buddy::Order::new(12);

/// # Safety
/// This function should only be called once and only during the kernel
/// initialization process.
///
/// # Panics
/// Panics if the heap cannot be allocated from the buddy allocator. This can
/// happen on a system with very little memory since the heap is allocated as a
/// single contiguous block of memory of a fixed size.
#[init]
pub unsafe fn setup() {
    let physical = buddy::allocate(HEAP_ORDER, buddy::AllocationFlags::KERNEL)
        .expect("failed to allocate kernel heap");
    let heap = arch::page::translate(physical).unwrap();

    ALLOCATOR
        .lock()
        .init(heap.as_mut_ptr::<u8>(), HEAP_ORDER.size());
}
