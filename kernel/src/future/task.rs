use crate::future::{executor::Executor, waker::Waker};
use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

/// A task that can be executed by an executor.
pub struct Task<'a> {
    /// The executor that owns the task.
    executor: &'a Executor<'a>,

    /// The future that the task is running.
    future: Pin<Box<dyn Future<Output = ()> + Send>>,

    /// The waker of the task.
    waker: Waker<'a>,

    /// The identifier of the task.
    id: Identifier,
}

impl<'a> Task<'a> {
    /// Creates a new task with the given executor and future.
    pub fn new(
        executor: &'a Executor<'a>,
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> Self {
        let id = Identifier::generate();
        let waker = Waker::new(executor.ready_queue(), id);
        Self {
            executor,
            future,
            waker,
            id,
        }
    }

    /// Polls the task and returns whether it has completed or not.
    pub fn poll(&mut self) -> core::task::Poll<()> {
        // SAFETY: This is safe because we have an exclusive access to the
        // task and by extension the waker. We also make sure that the
        // RawWaker is not used after this function returns, and that no
        // mutable references to the waker are created while the RawWaker
        // is in use.
        let waker = unsafe { core::task::Waker::from_raw(self.waker.raw()) };
        let context = &mut core::task::Context::from_waker(&waker);
        self.future.as_mut().poll(context)
    }

    /// Returns the executor that owns the task.
    #[must_use]
    pub const fn executor(&'a self) -> &'a Executor<'a> {
        self.executor
    }

    /// Returns the identifier of the task.
    #[must_use]
    pub const fn id(&self) -> Identifier {
        self.id
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Identifier {
    id: usize,
}

impl Identifier {
    /// Creates a new task identifier.
    pub fn generate() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl From<Identifier> for usize {
    fn from(id: Identifier) -> usize {
        id.id
    }
}
