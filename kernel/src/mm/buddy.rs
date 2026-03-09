use core::{iter::Step, ptr::NonNull};

use bitflags::bitflags;
use macros::init;

use crate::{
    arch::{
        self,
        addr::{AllMemory, Kernel, PAGE_SHIFT, Physical, Virtual},
    },
    library::lock::spin::Spinlock,
    mm::page::{self, Page},
};

/// A block order in the buddy allocator. The order determines the size of the
/// block, with a block of order `n` corresponding to a block of 2^n pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Order(u8);

impl Order {
    /// The maximum order of the buddy allocator, which determines the largest
    /// block size that can be allocated. Currently, the maximum order is set
    /// to 12, which corresponds to a block size of 16 MiB (assuming a page
    /// size of 4 KiB).
    pub const MAX: Self = Self(12);

    /// Create a new `Order` from the given order value.
    ///
    /// # Panics
    /// This function will panic if the provided order value exceeds the maximum
    /// allowed order defined by `Order::MAX`.
    #[must_use]
    pub const fn new(order: u8) -> Self {
        assert!(order <= Self::MAX.0);
        Self(order)
    }

    /// Create a new `Order` that is the nearest order for allocating a block
    /// of the given number of pages.
    ///
    /// # Panics
    /// This function will panic if the resulting order exceeds the maximum
    /// allowed order defined by `Order::MAX`.
    #[must_use]
    pub const fn nearest(pages: page::Count) -> Self {
        Self::new(Self::nearest_order(pages))
    }

    /// Create a new `Order` from the given order value. This function returns
    /// `None` if the order value exceeds the maximum allowed order, ensuring
    /// that the resulting `Order` is always within bounds.
    #[must_use]
    pub const fn try_new(order: u8) -> Option<Self> {
        if order > Self::MAX.0 {
            None
        } else {
            Some(Self(order))
        }
    }

    /// Create a new `Order` that is the nearest order for allocating a block
    /// of the given number of pages. This function returns `None` if the
    /// resulting order exceeds the maximum allowed order.
    #[must_use]
    pub const fn try_nearest(pages: page::Count) -> Option<Self> {
        Self::try_new(Self::nearest_order(pages))
    }

    /// Calculate the nearest order for allocating a block of the given number
    /// of pages. However, the buddy allocator may not support the resulting
    /// order if it exceeds `MAX_ORDER`.
    #[must_use]
    pub const fn nearest_order(pages: page::Count) -> u8 {
        if pages.0 <= 1 {
            return 0;
        }

        // An order is always guaranteed to fit in a `u8` since an order of 255
        // would correspond to a block size of 2^255 pages, which would never
        // exist in a desktop computer (famous last words ?).
        #[allow(clippy::cast_possible_truncation)]
        return (usize::BITS - (pages.0 - 1).leading_zeros()) as u8;
    }

    /// Check if this order is the maximum order supported by the buddy
    /// allocator. If this function returns `true`, it means that the order
    /// is equal to `Order::MAX`.
    #[must_use]
    pub const fn is_last(self) -> bool {
        self.0 == Self::MAX.0
    }

    /// Get the previous order, which corresponds to a block size that is half
    /// the size of the current order.
    ///
    /// # Panics
    /// This function will panic if called on the minimum order (order 0).
    #[must_use]
    pub const fn prev(self) -> Self {
        Self(self.0.checked_sub(1).unwrap())
    }

    /// Get the next order, which corresponds to a block size that is double
    /// the size of the current order.
    ///
    /// # Panics
    /// This function will panic if the resulting order exceeds the maximum
    /// allowed order defined by `Order::MAX`.
    #[must_use]
    pub const fn next(self) -> Self {
        Self::new(self.0 + 1)
    }

    /// Calculate the number of pages in a block of the given order.
    #[must_use]
    pub const fn pages(self) -> page::Count {
        page::Count(1 << self.0)
    }

    /// Calculate the size in bytes of a block of the given order.
    #[must_use]
    pub const fn size(self) -> usize {
        self.pages().to_bytes()
    }
}

impl From<Order> for u8 {
    #[inline]
    fn from(order: Order) -> Self {
        order.0
    }
}

impl From<Order> for usize {
    #[inline]
    fn from(order: Order) -> Self {
        order.0 as usize
    }
}

impl Step for Order {
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        if start > end {
            (0, None)
        } else {
            let steps = usize::from(*end) - usize::from(*start);
            (steps, Some(steps))
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Self::try_new(start.0.checked_add(u8::try_from(count).ok()?)?)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Self::try_new(start.0.checked_sub(u8::try_from(count).ok()?)?)
    }
}

bitflags! {
    /// Flags that can be used to specify additional options when allocating
    /// memory with the buddy allocator.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct AllocationFlags : u32 {
        /// The memory will be allocated for use by the kernel itself.
        const KERNEL = 1 << 0;
    }
}

/// An intrusive doubly linked list used to represent the free lists of the
/// buddy allocator. Each node in the list represents a free block of memory
/// of a certain order, and the address of the node corresponds to the start
/// of the free block.
///
/// # Safety
/// The free list uses an intrusive linked list design, which is highly
/// efficient for the buddy allocator's use case since it allows us to remove
/// and insert nodes in the free list in O(1) time without needing to allocate
/// additional memory for list nodes. The tradeoff is that double linked lists
/// are unsafe to manipulate in Rust, and we must rely on careful design and
/// safety contracts to ensure that we do not introduce undefined behavior.
#[derive(Debug)]
pub struct FreeList {
    head: Option<NonNull<FreeListNode>>,
}

/// SAFETY: `FreeList` is safe to send between threads since it "owns" the
/// nodes in the free list and is responsible for ensuring that all operations
/// on the free list are performed safely.
unsafe impl Send for FreeList {}

impl FreeList {
    /// Create a new empty `FreeList` with no nodes.
    #[must_use]
    pub const fn empty() -> Self {
        Self { head: None }
    }

    /// Pop a node from the head of the free list and return it.
    ///
    /// Returns `None` if the free list is empty.
    pub fn pop(&mut self) -> Option<NonNull<FreeListNode>> {
        // SAFETY: We are the sole owner of the free list and its nodes, and
        // since we have a mutable reference to the free list, we can safely
        // modify the free list and its nodes without worrying about aliasing
        // or concurrent access.
        //
        // Additionally, since we have ensured that all nodes pushed onto the
        // free list are valid and properly initialized (through the safety
        // contract of `push`), we can safely access the `next` and `prev`
        // pointers of the nodes without risking undefined behavior.
        unsafe {
            let mut node = self.head?;
            let next = node.as_ref().next;
            if let Some(mut head) = next {
                head.as_mut().prev = None;
            }

            node.as_mut().next = None;
            node.as_mut().prev = None;
            self.head = next;
            Some(node)
        }
    }

    /// Push the given `node` to the head of the free list.
    ///
    /// # Safety
    /// The caller must ensure that no other aliasing references to the `node`
    /// exist and that we are the sole owner of the `node` being pushed onto
    /// the free list. Additionally, the caller must ensure that the `node` is
    /// properly aligned, correctly initialized and will outlive its presence
    /// in the free list.
    pub unsafe fn push(&mut self, mut node: NonNull<FreeListNode>) {
        node.as_mut().next = self.head;
        node.as_mut().prev = None;
        if let Some(mut head) = self.head {
            head.as_mut().prev = Some(node);
        }
        self.head = Some(node);
    }

    /// Remove the given `node` from the free list.
    ///
    /// # Safety
    /// The caller must ensure that we have an exclusive reference to the
    /// `node` being removed from the free list. Additionally, the node must
    /// be inserted in this free list.
    ///
    /// Calling this function with a node that inserted in a different free
    /// list is undefined behavior.
    pub unsafe fn remove(&mut self, node: &mut FreeListNode) {
        if let Some(mut prev) = node.prev {
            prev.as_mut().next = node.next;
        } else {
            // Here, the node being removed is the head of the free list. We
            // need to update the head pointer to point to the next node in
            // the free list.
            self.head = node.next;
        }

        if let Some(mut next) = node.next {
            next.as_mut().prev = node.prev;
        }

        node.next = None;
        node.prev = None;
    }

    /// Debug function to print the contents of the free list for a given
    /// `order`.
    pub fn debug(&mut self, order: usize) {
        // SAFETY: We have an exclusive reference to the free list, which
        // guarantees that we have exclusive access to the nodes in the free
        // list and can safely create references to the nodes and access
        // their fields without undefined behavior.
        unsafe {
            let mut current = self.head;
            log::debug!("Free list of order {order}:");
            while let Some(node) = current {
                let start = node.as_ref().physical_head();
                let end = Physical::new(usize::from(start) + node.as_ref().order.size());
                log::debug!("\t {} - {}", start, end);
                current = node.as_ref().next;
            }
        }
    }
}

/// A node in the free list of the buddy allocator, representing a free block
/// of memory of a certain order.
#[derive(Debug)]
pub struct FreeListNode {
    prev: Option<NonNull<FreeListNode>>,
    next: Option<NonNull<FreeListNode>>,
    order: Order,
}

impl FreeListNode {
    /// Create a new `FreeListNode` with no previous or next nodes with the
    /// given `order`.
    #[must_use]
    pub const fn new(order: Order) -> Self {
        Self {
            prev: None,
            next: None,
            order,
        }
    }

    /// Create a new `FreeListNode` at the given `address` with the provided
    /// `order`.
    ///
    /// # Safety
    /// The caller must ensure that there is no other aliasing reference to the
    /// memory at the given `address`.
    ///
    /// # Panics
    /// Panics if the provided `address` does not belong to the HHDM region
    /// (High Half Direct Mapping) or if the `address` is not properly aligned.
    #[must_use]
    pub unsafe fn new_at(address: Virtual<Kernel>, order: Order) -> NonNull<Self> {
        assert!(address.is_aligned(order.size()));
        assert!(arch::page::in_hhdm(address));

        let node = NonNull::<FreeListNode>::from(address);
        node.as_ptr().write(Self::new(order));
        node
    }

    /// Create a new `FreeListNode` from the given physical address that
    /// corresponds to the head of an allocated buddy block that has been
    /// allocated with the [`allocate`] function but is now being freed.
    ///
    /// # Panics
    /// Panics if the provided `physical` address is not page aligned, does
    /// not correspond to a valid address in memory, if the address does not
    /// correspond to the head of an allocated buddy block, or if the buddy
    /// block is still in use.
    #[must_use]
    pub fn free_block_at(physical: Physical<AllMemory>) -> NonNull<Self> {
        let mut page = page::metadata().from_address(physical).lock();
        if let Page::UsedBuddyBlockHead {
            ref mut usage,
            order,
        } = *page
        {
            assert!(usage.dispose(), "Trying to free a block still in use");
            let address = arch::page::translate(physical).unwrap();

            // SAFETY: We have ensured that the provided `physical` address
            // is valid and that we are the owner of the block corresponding
            // to that address.
            unsafe { FreeListNode::new_at(address, order) }
        } else {
            panic!("Try to free a block that is not the head of a buddy block");
        }
    }

    /// Get the buddy node of this node for the given `order`.
    ///
    /// This is the caller responsibility to ensure that the buddy node is
    /// valid and can be safely dereferenced.
    #[must_use]
    pub fn buddy(&self, order: Order) -> NonNull<Self> {
        // SAFETY: The buddy address of a valid node is guaranteed to be
        // non-null since null pointers are not valid kernel addresses.
        unsafe {
            NonNull::new_unchecked(
                (core::ptr::from_ref::<Self>(self).addr()
                    ^ (1 << (usize::from(order.0) + PAGE_SHIFT))) as *mut Self,
            )
        }
    }

    /// Get a pointer to the next block of memory of the given order. This does
    /// not give us the next block in the free list, but rather the next block
    /// of memory that is adjacent to the current block and has the same order.
    ///
    /// The caller is responsible for ensuring that the next block of memory is
    /// free and contains a valid `FreeListNode` that can be dereferenced.
    ///
    /// # Panics
    /// Panics if the next block of memory does not belong to the HHDM region
    /// (High Half Direct Mapping).
    #[must_use]
    pub fn next_block(&self, order: Order) -> NonNull<Self> {
        let next = Virtual::<Kernel>::from_ref(self) + order.size();
        assert!(arch::page::in_hhdm(next));
        NonNull::from(next)
    }

    /// Get the physical address corresponding to the head of the buddy block
    /// represented by this `FreeListNode`.
    ///
    /// # Panics
    /// Panics if the virtual address of this `FreeListNode` does not belong
    /// to the HHDM region (High Half Direct Mapping). This should never happen
    /// since the buddy allocator relies on the fact that the physical memory
    /// can be directly accessed without needing to be mapped explicitly.
    #[must_use]
    pub fn physical_head(&self) -> Physical<AllMemory> {
        arch::page::from_hhdm(Virtual::<Kernel>::from_ref(self))
            .expect("The node's virtual address should always be valid")
    }

    /// Get the virtual address corresponding to the head of the buddy block
    /// represented by this `FreeListNode`.
    #[must_use]
    pub fn base_address(&self) -> Virtual<Kernel> {
        Virtual::<Kernel>::from_ref(self)
    }
}

/// The number of buckets in the buddy allocator.
pub const BUCKET_COUNT: usize = Order::MAX.0 as usize + 1;

/// The buddy allocator maintains a free list of blocks for each order, where
/// each block is represented by a `Block` struct. The `BUCKETS` array holds
/// the head of the free list for each order.
static BUCKETS: [Spinlock<FreeList>; BUCKET_COUNT] =
    [const { Spinlock::new(FreeList::empty()) }; BUCKET_COUNT];

/// Initialize the buddy allocator by populating the free lists with the blocks
/// of memory that are currently free according to the page metadata.
///
/// # Safety
/// This function should only be called once during the kernel initialization
/// phase, and it must be called after the page metadata table has been set up.
#[init]
pub unsafe fn setup() {
    page::metadata()
        .table()
        .iter()
        .enumerate()
        .filter(|(_, page)| matches!(*page.lock(), Page::UsedBuddyBlockHead { .. }))
        .for_each(|(index, _)| free(Physical::from_frame_index(index)));
}

/// Allocate a contiguous block of physical memory of the given `order`. If no
/// block of the requested order is available, return `None`.
///
/// # Panics
/// This function will panic only if it encounters an inconsistency in the
/// buddy allocator's data structures, such as finding a block in the free
/// list that does not have the expected order or if the page metadata of a
/// block being split does not have the expected state.
///
/// These panics indicate a bug in the buddy allocator implementation or in
/// the kernel's memory management subsystem, and they should never happen
/// under normal operation.
#[must_use]
pub fn allocate(requested_order: Order, flags: AllocationFlags) -> Option<Physical<AllMemory>> {
    let kernel = flags.contains(AllocationFlags::KERNEL);
    // Find a free block of memory large enough to satisfy the allocation
    // request. If we find such a block, we need to update the page metadata
    // of the block before dropping the lock on the free list to ensure that
    // the block will not be merged with other free blocks while we are still
    // using it to satisfy the allocation request.
    // SAFETY: The node returned by `pop` is guaranteed to not be aliased by
    // any other reference since it has been removed from the free list.
    let free = unsafe {
        BUCKETS
            .iter()
            .skip(usize::from(requested_order))
            .find_map(|free_list| {
                let mut free_list = free_list.lock();
                let mut node = free_list.pop()?;
                page::metadata()
                    .from_address(node.as_mut().physical_head())
                    .lock()
                    .change_state(Page::UsedBuddyBlockHead {
                        usage: page::UsageMetadata::used(kernel),
                        order: requested_order,
                    });
                Some(node)
            })?
            .as_mut()
    };

    // We found a block that is big enough to satisfy the allocation request,
    // but the block may be larger than the requested size. We need to split
    // the block into smaller blocks until we reach the desired order.
    for i in requested_order..free.order {
        // IMPORTANT: We need to lock the free list of the current order before
        // splitting the block to prevent a race condition where another thread
        // could merge the block with its buddy block after we split it but
        // before we push the buddy block onto the free list.
        let mut free_list = get_free_list(i).lock();
        let node = {
            // SAFETY: The next block is guaranteed to be valid, properly aligned
            // and can be safely converted to a mutable reference since the buddy
            // block is currently not in use. Furthermore, the free list node is
            // initialized to a sane state before being dereferenced.
            let block = unsafe {
                free.next_block(i)
                    .as_uninit_mut()
                    .write(FreeListNode::new(i))
            };
            let physical = block.physical_head();

            // Get the page corresponding to the newly created buddy block and
            // update its metadata to reflect that it is now the head of a buddy
            // block of order `i`.
            let mut page = page::metadata().from_address(physical).lock();
            if let Page::BuddyBlockPage = *page {
                page.change_state(Page::FreeBuddyBlockHead { order: i });
            } else {
                panic!("The node's physical address should always correspond to a valid page");
            }

            NonNull::from_mut(block)
        };

        // SAFETY: The buddy node is valid, properly aligned and can be safely
        // pushed onto the free list since the buddy block is currently not in
        // use and we are the exclusive owner of the buddy node.
        unsafe {
            free_list.push(node);
        }
    }

    Some(free.physical_head())
}

/// Free a block of memory allocated by `allocate` by providing the physical
/// address corresponding to the head of the buddy block.
///
/// # Panics
/// Panics if the provided `physical` address is not page aligned, does not
/// correspond to the head of a buddy block that was allocated with the
/// [`allocate`] function, or if the block is still used elsewhere (has a
/// usage count greater than 1).
pub fn free(physical: Physical<AllMemory>) {
    let (node, base, order) = {
        // SAFETY: We have an exclusive access to the node since the memory block
        // corresponding to the node is being freed and therefore should not be in
        // use anymore.
        let mut node = unsafe { FreeListNode::free_block_at(physical).as_mut() };
        let mut base = node.physical_head();
        let mut order = node.order;

        while let Some(mut bucket) = can_coalesce(base, order) {
            // SAFETY: We checked that the buddy block of the node can be
            // coalesced, meaning that there is a node at the buddy address
            // that is properly aligned and initialized. Since we have locked
            // the free list corresponding to the buddy block's order, we can
            // assume that we have an exclusive reference to the buddy node.
            let mut buddy = unsafe { node.buddy(order).as_mut() };

            // Remove the buddy node from its free list.
            // SAFETY: We still have an exclusive reference to the buddy node as
            // stated above, and we remove the buddy node from the free list
            // corresponding to its order
            unsafe {
                bucket.remove(buddy);
            }

            // To maintain the invariant that the node with the smaller physical
            // address is the one that remains after coalescing, we need to compare
            // and potentially swap the current node reference with the buddy node
            // reference. This ensures that after coalescing, the node reference
            // always points to the block with the smaller physical address.
            if core::ptr::from_mut(buddy) < core::ptr::from_mut(node) {
                core::mem::swap(&mut buddy, &mut node);
                base = node.physical_head();
            }

            // Change the state of the first page of the buddy block that we are
            // coalescing with to reflect that it is no longer the head of a
            // buddy block, but rather a regular page that is part of a larger
            // buddy block.
            page::metadata()
                .from_address(buddy.physical_head())
                .lock()
                .change_state(Page::BuddyBlockPage);
            order = order.next();
        }

        // We have coalesced the current node with all of its buddy blocks that
        // can be coalesced or we have reached the maximum order.
        // Update the page metadata of the coalesced block's head page and the
        // `order` field of the node to reflect the new order of the coalesced
        // block.
        node.order = order;
        (NonNull::from_mut(node), base, order)
    };

    // Lock the free list corresponding to the final order of the coalesced
    // block while updating the page metadata of the block's head page to
    // avoid races conditions where another thread could try to coalesce the
    // block with its buddy block before we update the page metadata.
    let mut free_list = get_free_list(order).lock();
    page::metadata()
        .from_address(base)
        .lock()
        .change_state(Page::FreeBuddyBlockHead { order });

    // SAFETY: The node is properly initialized and correctly aligned, not
    // aliased by any other reference and we are the exclusive owner of the
    // node pushing it onto the free list.
    unsafe {
        free_list.push(node);
    }
}

/// Debug function to print the contents of all the free lists in the buddy
/// allocator for debugging purposes.
pub fn print_debug() {
    for (i, bucket) in BUCKETS.iter().enumerate() {
        bucket.lock().debug(i);
    }
}

/// Check if a hypothetical buddy block with the given `physical` address and
/// `order` could be coalesced with its buddy block in the current state of the
/// buddy allocator.
///
/// If the buddy block can be coalesced, this function returns the free list
/// corresponding to the given `order` with the lock held, which the caller
/// can use to safely remove the buddy block from the free list without race
/// conditions.
fn can_coalesce(
    physical: Physical<AllMemory>,
    allocation_order: Order,
) -> Option<spin::MutexGuard<'static, FreeList>> {
    // Early return if the given `order` is the maximum order to avoid
    // unnecessary locking of the free list and other computations.
    if allocation_order.is_last() {
        return None;
    }

    // Lock the free list corresponding to the given `order` before checking if
    // the buddy block can be coalesced to prevent a race condition where
    // another thread could modify the free list after we checked if the buddy
    // block can be coalesced, potentially leading to undefined behavior.
    let addr = buddy_address(physical, allocation_order);
    let bucket = get_free_list(allocation_order).lock();
    if let Some(buddy) = page::metadata().try_from_address(addr)
        && let Page::FreeBuddyBlockHead { order } = *buddy.lock()
        && order == allocation_order
    {
        return Some(bucket);
    }

    None
}

/// Calculate the buddy address of a block with the given `physical` address
/// and `order`. The buddy address is the physical address of the block that
/// is adjacent to the given block and has the same size (order) as the given
/// block.
#[inline]
#[must_use]
fn buddy_address(physical: Physical<AllMemory>, order: Order) -> Physical<AllMemory> {
    Physical::new(usize::from(physical) ^ (1 << (usize::from(order) + PAGE_SHIFT)))
}

/// Get the free list associated with the given `order`.
#[inline]
#[must_use]
fn get_free_list(order: Order) -> &'static Spinlock<FreeList> {
    // SAFETY: The `Order` type ensures that the `order` value is always within
    // bounds which guarantees that the index used to access the `BUCKETS`
    // array is always valid. Therefore, we can skip the bounds check when
    // accessing the `BUCKETS` array for a (very) small performance gain.
    unsafe { BUCKETS.get_unchecked(usize::from(order)) }
}
