use super::task::{self};
use core::task::{RawWaker, RawWakerVTable};
use crossbeam::queue::ArrayQueue;

static WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    // Clone
    |data| RawWaker::new(data.cast::<()>(), &WAKER_VTABLE),
    // Wake
    |data| unsafe {
        let waker = &*(data.cast::<Waker>());
        waker.queue.push(waker.id).expect("Queue is full");
    },
    // Wake by ref
    |data| unsafe {
        let waker = &*(data.cast::<Waker>());
        waker.queue.push(waker.id).expect("Queue is full");
    },
    // Drop
    |_| {},
);

/// A waker that can wake up a task.
#[derive(Debug)]
pub struct Waker<'a> {
    /// The que to push the task identifier to when waking
    /// up the task.
    queue: &'a ArrayQueue<task::Identifier>,

    /// The identifier of the task to wake up.
    id: task::Identifier,
}

impl<'a> Waker<'a> {
    /// Create a new waker.
    #[must_use]
    pub const fn new(
        queue: &'a ArrayQueue<task::Identifier>,
        id: task::Identifier,
    ) -> Self {
        Waker { queue, id }
    }

    /// Create a raw waker from the waker.
    ///
    /// # Safety
    /// The caller must ensure that the raw waker is not used after the
    /// waker is dropped. The caller must also ensure to not create
    /// mutable references to the waker while the raw waker is in use.
    #[must_use]
    pub(super) unsafe fn raw(&self) -> RawWaker {
        RawWaker::new(core::ptr::from_ref(self).cast::<()>(), &WAKER_VTABLE)
    }
}
