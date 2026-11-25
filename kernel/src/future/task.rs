use core::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, sync::Arc};

use crate::future::{executor::LocalExecutor, waker::LocalWaker};

pub struct LocalTask {
    executor: Arc<LocalExecutor>,
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    waker: Option<Arc<LocalWaker>>,
    id: Identifier,
}

impl LocalTask {
    /// Creates a new local task.
    pub fn new(
        executor: Arc<LocalExecutor>,
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> Self {
        Self {
            executor,
            future,
            waker: None,
            id: Identifier::generate(),
        }
    }

    /// Returns the identifier of the task.
    #[must_use]
    pub fn id(&self) -> Identifier {
        self.id
    }

    /// Polls the task and returns whether it has completed or not.
    ///
    /// The waker is created during the first poll and then reused for all
    /// subsequent polls, so that futures that compare wakers with
    /// [`core::task::Waker::will_wake`] can avoid cloning it again.
    pub fn poll(&mut self) -> core::task::Poll<()> {
        let waker =
            Arc::clone(self.waker.get_or_insert_with(|| {
                Arc::new(LocalWaker::new(Arc::clone(&self.executor), self.id))
            }));

        // Clear the `queued` flag before polling the future, so that a wake up
        // that happens during the poll will queue the task again.
        waker.clear_queued();

        let waker = waker.into();
        let mut context = core::task::Context::from_waker(&waker);
        self.future.as_mut().poll(&mut context)
    }
}

/// A unique identifier for a task.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Identifier(usize);

impl Identifier {
    /// Creates a new task identifier. The identifier is guaranteed to be
    /// unique across the entire kernel runtime.
    pub fn generate() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<usize> for Identifier {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

impl From<Identifier> for usize {
    fn from(id: Identifier) -> usize {
        id.0
    }
}

impl core::fmt::Display for Identifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}
