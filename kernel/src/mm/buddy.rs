use core::ptr::NonNull;

use macros::init;

use crate::{
    arch::{
        self,
        addr::{AllMemory, Kernel, PAGE_SHIFT, PAGE_SIZE, Physical, Virtual},
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
    /// allowed order defined by `Order::GLOBAL_MAX`.
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
    /// allowed order defined by `Order::GLOBAL_MAX`.
    #[must_use]
    pub const fn nearest(pages: page::Count) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self::new(Self::nearest_order(pages) as u8)
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
    pub const fn try_neareast(pages: page::Count) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)]
        Self::try_new(Self::nearest_order(pages) as u8)
    }

    /// Calculate the nearest order for allocating a block of the given number
    /// of pages. However, the buddy allocator may not support the resulting
    /// order if it exceeds `MAX_ORDER`.
    #[must_use]
    pub const fn nearest_order(pages: page::Count) -> u32 {
        if pages.0 <= 1 {
            return 0;
        }
        usize::BITS - (pages.0 - 1).leading_zeros()
    }

    /// Calculate the number of pages in a block of the given order.
    #[must_use]
    pub const fn pages(self) -> page::Count {
        page::Count(1 << self.0)
    }

    /// Calculate the size in bytes of a block of the given order.
    #[must_use]
    pub const fn size(self) -> usize {
        self.pages().0 * PAGE_SIZE
    }
}

pub struct FreeList {
    head: Option<NonNull<FreeListNode>>,
}

/// SAFETY: TODO
unsafe impl Send for FreeList {}

impl FreeList {
    #[must_use]
    pub const fn empty() -> Self {
        Self { head: None }
    }

    /// Pop a node from the head of the free list and return it. Returns `None`
    /// if the free list is empty.
    pub fn pop(&mut self) -> Option<NonNull<FreeListNode>> {
        // SAFETY: We are the sole owner of the free list and its nodes, and
        // since we have a mutable reference to the free list, we can safely
        // modify the free list and its nodes without worrying about aliasing
        // or concurrent access.
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
    /// properly aligned, correctly initialized
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

    pub fn debug(&self) {
        unsafe {
            let mut current = self.head;
            log::debug!(
                "Free list of order {}:",
                current.map_or(0, |node| node.as_ref().order.0)
            );
            while let Some(node) = current {
                let start = node.as_ref().physical_head();
                let end = Physical::new(usize::from(start) + node.as_ref().order.size());
                log::debug!("\t {} - {}", start, end);
                current = node.as_ref().next;
            }
        }
    }
}

pub struct FreeListNode {
    prev: Option<NonNull<FreeListNode>>,
    next: Option<NonNull<FreeListNode>>,
    order: Order,
}

/// SAFETY: TODO
unsafe impl Send for FreeListNode {}

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

    /// Create a new `FreeListNode` from the given physical address that
    /// corresponds to the head of an allocated buddy block.
    ///
    /// # Panics
    /// Panics if the provided `physical` address is not page aligned, does not
    /// correspond to a valid address in memory, if the address does not
    /// correspond to the head of an allocated buddy block, or if the buddy
    /// block is still in use.
    #[must_use]
    pub fn from_physical(physical: Physical<AllMemory>) -> NonNull<Self> {
        assert!(physical.is_aligned(PAGE_SIZE));
        let mut page = page::table()
            .get(physical.frame_idx())
            .expect("Trying to free a block that does not exist in memory")
            .lock();

        if let Page::UsedBuddyBlockHead { order, usage } = &mut *page {
            // Here, the caller is trying to free a block that is the head of
            // an allocated buddy block. Therefore, the data contained in that
            // block are no longer needed, and we can safely create a new
            // `FreeListNode` at the start of the block to represent the freed
            // block in the buddy allocator's
            assert!(usage.dispose(), "Trying to free a block still in use");

            let order = *order;
            *page = Page::FreeBuddyBlockHead { order };

            // SAFETY: `arch::page::translate` is guaranteed to return a valid
            // virtual kernel address. Since kernel addresses are in the high
            // half of the address space, they are guaranteed to be non-null.
            let node = unsafe {
                NonNull::new_unchecked(
                    arch::page::translate(physical)
                        .unwrap()
                        .as_mut_ptr::<Self>(),
                )
            };

            // SAFETY: We have ensured that the caller is the sole owner of the
            // block and that the pointer is properly aligned and not null.
            unsafe {
                node.as_ptr().write(Self::new(order));
            }

            node
        } else {
            // Here, the caller tried to free a block that is not the head of
            // a allocated buddy block. The caller likely made a mistake and
            // gave an invalid address. The kernel code is buggy, and we must
            // panic to prevent further corruption of the buddy allocator's
            // state
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
                    ^ (1 << (usize::from(order.0) + PAGE_SIZE.trailing_zeros() as usize)))
                    as *mut Self,
            )
        }
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

    /// Check if this node is currently inserted in a free list or not.
    #[must_use]
    pub fn is_inserted(&self) -> bool {
        self.prev.is_some() || self.next.is_some()
    }

    /// Remove this node from the free list corresponding to its `order`,
    /// returning the `Order` of the block that this node represents. If the
    /// node is not currently inserted in the free list, this function does
    /// nothing.
    ///
    /// # Safety
    /// The caller must ensure that this node is currently inserted in the free
    /// list corresponding to its `order`.
    pub unsafe fn remove_itself(&mut self) -> Order {
        if self.is_inserted() {
            get_free_list(self.order).lock().remove(self);
        }
        self.order
    }
}

/// The number of buckets in the buddy allocator.
pub const BUCKET_COUNT: usize = Order::MAX.0 as usize + 1;

/// The buddy allocator maintains a free list of blocks for each order, where
/// each block is represented by a `Block` struct. The `BUCKETS` array holds
/// the head of the free list for each order.
static BUCKETS: [Spinlock<FreeList>; BUCKET_COUNT] =
    [const { Spinlock::new(FreeList::empty()) }; BUCKET_COUNT];

/// # Safety
/// This function should only be called once during the kernel initialization
/// phase, and it must be called after the page metadata table has been set up.
#[init]
pub unsafe fn setup() {
    // Iterate over all free pages, change their page metadata to
    // `Page::UsedBuddyBlockHead` with an order of 0, and call `free` on each
    // page to effectively reclaim them into the buddy allocator's free lists.
    // This avoids the need for a separate initialization step to populate the
    // free lists with the free memory pages.

    for (i, mut page) in page::table()
        .iter()
        .enumerate()
        .map(|(i, entry)| (i, entry.lock()))
        .filter(|(_, page)| matches!(**page, Page::Free))
    {
        page.change_state(Page::UsedBuddyBlockHead {
            usage: page::UsageMetadata::used(false),
            order: Order::new(0),
        });
    }

    for (i, _) in page::table()
        .iter()
        .enumerate()
        .filter(|(_, page)| matches!(*page.lock(), Page::UsedBuddyBlockHead { .. }))
    {
        free(Physical::new(i * PAGE_SIZE));
    }

    for bucket in BUCKETS.iter() {
        bucket.lock().debug();
    }

    // Allocate a block of order 12
    let block = allocate(Order::new(12)).expect("Failed to allocate block of order 6");
    log::info!(
        "Allocated block of order 6 at physical address {:#x}",
        block
    );
}

/// # Panics
/// TODO
#[must_use]
pub fn allocate(order: Order) -> Option<Physical<AllMemory>> {
    // SAFETY: The node returned by `pop` is guaranteed to not be aliased by
    // any other reference since it has been removed from the free list.
    let node = unsafe {
        BUCKETS
            .iter()
            .skip(usize::from(order.0))
            .find_map(|free_list| free_list.lock().pop())?
            .as_mut()
    };

    // We found a block that is big enough to satisfy the allocation request,
    // but the block may be larger than the requested size. We need to split
    // the block into smaller blocks until we reach the desired order.
    for i in (order.0..node.order.0).rev().map(|i| Order::new(i - 1)) {
        // SAFETY: The buddy address is guaranted to be valid, properly aligned
        // and can be safely converted to a mutable reference since the buddy
        // block is currently not in use. The free list node is initialized to
        // a sane state before being dereferenced.
        let buddy = unsafe { node.buddy(i).as_uninit_mut().write(FreeListNode::new(i)) };

        // SAFETY: The buddy node is valid, properly aligned and can be safely
        // pushed onto the free list since the buddy block is currently not in
        // use and we are the exclusive owner of the buddy node.
        unsafe {
            get_free_list(i).lock().push(NonNull::from_mut(buddy));
        }

        // Get the page corresponding to the newly created buddy block and
        // update its metadata to reflect that it is now the head of a buddy
        // block of order `i`.
        let mut page = page::table()
            .get(node.physical_head().frame_idx())
            .expect("The node's physical address should always be valid")
            .lock();
        if let Page::BuddyBlockPage { .. } = *page {
            *page = Page::UsedBuddyBlockHead {
                usage: page::UsageMetadata::used(false),
                order: i,
            };

            // Update the page metadata of the buddy block's head page since
            // the head of the buddy block they belong to has changed.
            for j in 1..i.pages().0 {
                page::table()
                    .get(node.physical_head().frame_idx() + j)
                    .expect("The node's physical address should always be valid")
                    .lock()
                    .change_state(Page::UsedBuddyBlockHead {
                        usage: page::UsageMetadata::used(false),
                        order: i,
                    });
            }
        } else {
            panic!("The node's physical address should always correspond to a valid page");
        }
    }

    // Finally, we have reached the desired order for the allocated block. We
    // need to update the page metadata of the block's head page to reflect
    // that it is now allocated and is the head of a buddy block of the
    // requested order.
    page::table()
        .get(node.physical_head().frame_idx())
        .expect("The node's physical address should always be valid")
        .lock()
        .change_state(Page::UsedBuddyBlockHead {
            usage: page::UsageMetadata::used(true),
            order,
        });

    Some(node.physical_head())
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
    // SAFETY: The node is guaranteed to be inserted in the free list
    // corresponding to its order since it just was retrived from the buddy
    // block's head page.
    let node = unsafe { FreeListNode::from_physical(physical).as_mut() };
    let order = node.order;

    // Update the page metadata of the buddy block's head page to reflect that
    // it is now free and is the head of a free buddy block of the given order.
    page::table()
        .get(physical.frame_idx())
        .expect("The physical address should always correspond to a valid page")
        .lock()
        .change_state(Page::FreeBuddyBlockHead { order });

    // Update the page metadata of the buddy block's head page since the state
    // of the buddy block they belong to has changed.
    for i in 1..order.pages().0 {
        page::table()
            .get(physical.frame_idx() + i)
            .expect("The physical address should always correspond to a valid page")
            .lock()
            .change_state(Page::BuddyBlockPage {
                page: physical.frame_idx(),
            });
    }

    coalesce(node);
}

/// # Panics
/// TODO
pub fn coalesce(mut node: &mut FreeListNode) {
    let mut base = node.physical_head();
    let mut order = node.order;

    while can_coalesce(base, order) {
        let mut buddy = unsafe {
            NonNull::new(
                arch::page::translate(buddy_address(base, order))
                    .expect("Buddy block is not in HHDM")
                    .as_mut_ptr::<FreeListNode>(),
            )
            .unwrap()
            .as_mut()
        };

        // If the buddy address is lower than the base address, swap both
        // pointers so that the base pointer always points to the lower
        // address.
        if buddy.physical_head() < node.physical_head() {
            core::mem::swap(&mut buddy, &mut node);
            base = node.physical_head();
        }

        // Remove the buddy node from its free list since we are going to
        // coalesce it with the current node.
        unsafe {
            buddy.remove_itself();
        }

        // Change the state of all pages in the buddy block to reflect that
        // they now belong to a larger buddy block of order `order + 1` with
        // the head page at `base`.
        for i in 0..order.pages().0 {
            page::table()
                .get(buddy.physical_head().frame_idx() + i)
                .expect("The physical address should always correspond to a valid page")
                .lock()
                .change_state(Page::BuddyBlockPage {
                    page: base.frame_idx(),
                });
        }

        // If we are at the maximum order, we cannot coalesce any further since
        // the buddy allocator does not support blocks larger than the current
        // order and we must break out of the loop.
        if order == Order::MAX {
            break;
        }
        order = Order::new(order.0 + 1);
    }

    node.order = order;
    page::table()
        .get(base.frame_idx())
        .expect("The physical address should always correspond to a valid page")
        .lock()
        .change_state(Page::FreeBuddyBlockHead { order });

    // Finally, we have coalesced the block as much as possible. We need to
    // push the resulting node representing the coalesced block onto the free
    // list corresponding to the final order of the coalesced block.
    unsafe {
        get_free_list(order).lock().push(NonNull::from_mut(node));
    }
}

fn can_coalesce(physical: Physical<AllMemory>, order: Order) -> bool {
    if let Page::FreeBuddyBlockHead { .. } = page::table()
        .get(physical.frame_idx())
        .expect("The physical address should always correspond to a valid page")
        .lock()
        .clone()
    {
        if let Some(buddy) = page::table().get(buddy_address(physical, order).frame_idx()) {
            if let Page::FreeBuddyBlockHead { order: buddy_order } = buddy.lock().clone() {
                return order == buddy_order;
            }
        }
    }

    false
}

#[inline]
#[must_use]
fn buddy_address(physical: Physical<AllMemory>, order: Order) -> Physical<AllMemory> {
    Physical::new(usize::from(physical) ^ (1 << (usize::from(order.0) + PAGE_SHIFT)))
}

#[inline]
#[must_use]
fn get_free_list(order: Order) -> &'static Spinlock<FreeList> {
    // SAFETY: The `Order` type ensures that the `order` value is always within
    // bounds which guarantees that the index used to access the `BUCKETS`
    // array is always valid. Therefore, we can skip the bounds check when
    // accessing the `BUCKETS` array for a (very) small performance gain.
    unsafe { BUCKETS.get_unchecked(usize::from(order.0)) }
}
