use core::sync::atomic::{AtomicBool, Ordering};

use alloc::sync::Arc;

use crate::future::{self, executor::LocalExecutor};

/// A waker that can wake up a task.
pub struct LocalWaker {
    /// The executor that owns the task to wake up.
    executor: Arc<LocalExecutor>,

    /// Whether the task is already in the ready queue of its executor. This
    /// avoids pushing the same task multiple times into the ready queue if it
    /// is woken up several times before being polled.
    queued: AtomicBool,

    /// The identifier of the task to wake up.
    id: future::task::Identifier,
}

impl LocalWaker {
    /// Create a new waker.
    #[must_use]
    pub fn new(executor: Arc<LocalExecutor>, id: future::task::Identifier) -> Self {
        LocalWaker {
            executor,
            queued: AtomicBool::new(false),
            id,
        }
    }

    /// Returns the executor that owns the task to wake up.
    #[must_use]
    pub fn executor(&self) -> &Arc<LocalExecutor> {
        &self.executor
    }

    /// Returns the identifier of the task to wake up.
    #[must_use]
    pub fn id(&self) -> future::task::Identifier {
        self.id
    }

    /// Marks the task as no longer being in the ready queue. This must be
    /// called by the executor right before polling the task.
    pub fn clear_queued(&self) {
        self.queued.store(false, Ordering::Release);
    }
}

impl alloc::task::Wake for LocalWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // The ready queue is locked with interrupts disabled, since a task can
        // be woken up from an interrupt handler (e.g. a timer callback) while
        // the executor of the same CPU is holding the lock.
        if !self.queued.swap(true, Ordering::AcqRel) {
            self.executor
                .ready_kernel_tasks
                .lock_irq_safe()
                .push_back(self.id);
        }
    }
}
