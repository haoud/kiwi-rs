use crate::{
    future::{executor::Executor, waker::Waker},
    time,
};
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

    /// The virtual runtime of the task. The virtual terminology is borrowed
    /// from the Completely Fair Scheduler (CFS) in Linux. It represents the
    /// amount of CPU time the task has consumed, in nanoseconds. However, it
    /// does not directly correspond to real time, as it depends on the lowest
    /// quantum of all tasks in the system when the task was created.
    vruntime: u64,

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
        vruntime: u64,
    ) -> Self {
        let id = Identifier::generate();
        let waker = Waker::new(executor.ready_ids(), id);
        Self {
            executor,
            future,
            vruntime,
            waker,
            id,
        }
    }

    /// Polls the task and returns whether it has completed or not. It also updates
    /// the virtual runtime of the task based on the time spent in the poll.
    #[allow(clippy::cast_possible_truncation)]
    pub fn poll(&mut self) -> core::task::Poll<()> {
        // SAFETY: This is safe because we have an exclusive access to the
        // task and by extension the waker. We also make sure that the
        // RawWaker is not used after this function returns, and that no
        // mutable references to the waker are created while the RawWaker
        // is in use.
        let waker = unsafe { core::task::Waker::from_raw(self.waker.raw()) };
        let context = &mut core::task::Context::from_waker(&waker);
        let (output, elapsed) = time::spent_into(|| self.future.as_mut().poll(context));
        self.vruntime += elapsed.as_nanos() as u64;
        output
    }

    /// Sets the virtual runtime of the task.
    pub(super) fn set_vruntime(&mut self, vruntime: u64) {
        self.vruntime = vruntime;
    }

    /// Returns the virtual runtime of the task.
    #[must_use]
    pub(super) fn vruntime(&self) -> u64 {
        self.vruntime
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

/// A unique identifier for a task.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Identifier(usize);

impl Identifier {
    /// Creates a new task identifier. The identifier is guaranteed to be unique
    /// across the entire kernel runtime.
    pub fn generate() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
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
