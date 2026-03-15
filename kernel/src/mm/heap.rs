use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Deref,
    ptr::NonNull,
};

use linked_list_allocator::Heap;
use macros::init;

use crate::{arch, library::lock::spin::Spinlock, mm::buddy};

/// A wrapper around the linked list allocator that provides a spinlock for
/// thread safety.
pub struct LockedHeap(Spinlock<Heap>);

impl LockedHeap {
    /// Creates a new empty `LockedHeap`.
    #[must_use]
    pub const fn empty() -> LockedHeap {
        LockedHeap(Spinlock::new(Heap::empty()))
    }
}

impl Deref for LockedHeap {
    type Target = Spinlock<Heap>;

    fn deref(&self) -> &Spinlock<Heap> {
        &self.0
    }
}

/// SAFETY: All safety requirements of the `GlobalAlloc` trait are upheld by
/// the `LockedHeap` implementation.
unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), core::ptr::NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .deallocate(NonNull::new_unchecked(ptr), layout);
    }
}

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
