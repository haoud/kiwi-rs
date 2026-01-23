use crate::{
    future::{self, executor::Executor, waker::Waker},
    ipc, time,
};
use alloc::{boxed::Box, sync::Arc};
use core::{
    future::Future,
    hash::Hash,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
use hashbrown::HashMap;
use spin::{Lazy, RwLock};

/// The local data associated with each task.
static TASK_LOCAL_DATA_MAP: Lazy<RwLock<HashMap<Identifier, LocalDataSet>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

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
    waker: Arc<Waker>,

    /// The identifier of the task.
    id: Identifier,
}

impl<'a> Task<'a> {
    /// Creates a new task with the given executor and future. It also creates
    /// the local data set for the task.
    pub fn new(
        executor: &'a Executor<'a>,
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
        vruntime: u64,
    ) -> Self {
        let id = Identifier::generate();
        let waker = Arc::new(Waker::new(Arc::clone(executor.ready_ids()), id));

        // Create the local data set for the task
        TASK_LOCAL_DATA_MAP
            .write()
            .insert(id, LocalDataSet::default());

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
        let waker = Arc::clone(&self.waker).into();
        let mut context = core::task::Context::from_waker(&waker);
        let (output, elapsed) = time::spent_into(|| self.future.as_mut().poll(&mut context));
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

impl Drop for Task<'_> {
    fn drop(&mut self) {
        // Remove the local data set for the task
        TASK_LOCAL_DATA_MAP.write().remove(&self.id);
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

/// The local data set associated with a task. This data is specific to each
/// task and is not shared between tasks, although a task can access the local
/// data of other tasks through interior mutability.
#[derive(Debug)]
pub struct LocalDataSet {
    /// A queue where this task can sleep waiting to receive an IPC message.
    pub ipc_receive_queue: future::wait::Queue,

    /// A queue where tasks that are waiting for a reply from this task can sleep.
    pub ipc_reply_queue: future::wait::Queue,

    /// A queue of tasks waiting to send IPC messages to this task.
    pub ipc_send_queue: future::wait::Queue,

    /// An incoming IPC message for the task.
    pub ipc_message: spin::Mutex<Option<Box<ipc::message::Message>>>,

    /// The reply message sent to this task.
    pub ipc_reply: spin::Mutex<Option<Box<ipc::message::Message>>>,

    /// The IPC state of the task.
    pub ipc_waiting_state: spin::Mutex<ipc::message::IpcWaitingState>,
}

impl Default for LocalDataSet {
    fn default() -> Self {
        Self {
            ipc_receive_queue: future::wait::Queue::new(),
            ipc_reply_queue: future::wait::Queue::new(),
            ipc_send_queue: future::wait::Queue::new(),
            ipc_message: spin::Mutex::new(None),
            ipc_reply: spin::Mutex::new(None),
            ipc_waiting_state: spin::Mutex::new(ipc::message::IpcWaitingState::None),
        }
    }
}

impl Drop for LocalDataSet {
    fn drop(&mut self) {
        // Poison queues to prevent any new tasks from sleeping on it,
        // then wake up all tasks waiting to send IPC messages to this task
        // or waiting for a reply from this task to prevent them from being
        // stuck forever.
        self.ipc_reply_queue.poison();
        self.ipc_reply_queue.wake_all();
        self.ipc_send_queue.poison();
        self.ipc_send_queue.wake_all();
    }
}

/// Checks if a task with the given identifier exists. It verifies the
/// existence of the local data set for the task, since the local data set is
/// created and destroyed along with the task itself.
pub fn exists(id: Identifier) -> bool {
    let map = TASK_LOCAL_DATA_MAP.read();
    map.contains_key(&id)
}

/// Executes a closure with access to the local data set of the task with
/// the given identifier. If the task does not exist, `None` is passed to the
/// closure.
///
/// Nested calls to this function are allowed, since the local data set is only
/// borrowed for read access. Mutating the local data set must be done through
/// interior mutability.
pub fn try_with_local_set_from<F, R>(id: Identifier, f: F) -> R
where
    F: FnOnce(Option<&LocalDataSet>) -> R,
{
    let map = TASK_LOCAL_DATA_MAP.read();
    let local_data_set = map.get(&id);
    f(local_data_set)
}

/// Executes a closure with access to the local data set of the task with
/// the given identifier. Nested calls to this function are allowed, since
/// the local data set is only borrowed for read access. Mutating the local
/// data set must be done through interior mutability.
///
/// # Panics
/// Panics if the task with the given identifier does not exist.
pub fn with_local_set_from<F, R>(id: Identifier, f: F) -> R
where
    F: FnOnce(&LocalDataSet) -> R,
{
    let map = TASK_LOCAL_DATA_MAP.read();
    let local_data_set = map.get(&id).expect("Task local data set not found");
    f(local_data_set)
}

/// Executes a closure with access to the local data set of the currently
/// running task. Nested calls to this function are allowed, since the local
/// data set is only borrowed for read access. Mutating the local data set must
/// be done through interior mutability.
///
/// # Panics
/// Panics if there is no currently running task.
pub fn with_current_local_set<F, R>(f: F) -> R
where
    F: FnOnce(&LocalDataSet) -> R,
{
    let current_id = future::executor::current_task_id().unwrap();
    with_local_set_from(current_id, f)
}
