use alloc::vec::Vec;
use hashbrown::HashMap;

use crate::future;

/// The table that keeps track for a given receiver, which tasks have sent
/// a message and are waiting for a reply.
static PENDING_MESSAGE_TASKS: spin::Once<spin::Mutex<HashMap<usize, Vec<usize>>>> =
    spin::Once::new();

/// The global message storage that holds messages sent between processes. Our
/// IPC mechanism is based on synchronous message passing, where a sender
/// sends a message to a receiver and waits for a reply.
static MESSAGES: spin::Once<spin::Mutex<HashMap<usize, Message>>> = spin::Once::new();

/// The global receiver queues storage that holds the wait queues for each
/// receiver process. This is used to wake up receivers when a new message
/// arrives for them.
static RECEIVERS_QUEUE: spin::Once<spin::Mutex<HashMap<usize, future::wait::Queue>>> =
    spin::Once::new();

/// Represents a message sent between processes.
#[derive(Debug, Clone)]
pub struct Message {
    /// The sender's process identifier.
    pub sender: usize,

    /// The receiver's process identifier.
    pub receiver: usize,

    /// The queue that the sender is using to wait for a reply. This is used
    /// to notify the sender when a reply is available.
    pub sender_queue: future::wait::Queue,

    /// The operation code of the message. This defines the type of request
    /// or action that the sender wants the receiver to perform. This could
    /// represent various operations like read, write, open, close, etc. And
    /// the interpretation of this field is up to the receiver process.
    pub operation: usize,

    /// The actual size of the payload data. This indicates how many bytes
    /// of the `payload` array are valid and should be processed by the
    /// receiver.
    pub payload_len: usize,

    /// The payload of the message. This is a fixed-size array to ensure
    /// that messages have a consistent size. If the actual payload is smaller
    /// than `MAX_PAYLOAD_SIZE`, the remaining bytes should be considered
    /// as padding and ignored.
    pub payload: [u8; Message::MAX_PAYLOAD_SIZE],
}

impl Message {
    /// The maximum size of the payload in bytes. This constant defines the
    /// upper limit for the amount of data that can be sent in a single
    /// message, ensuring that messages remain manageable in size.
    pub const MAX_PAYLOAD_SIZE: usize = 256;
}

/// Represents errors that can occur when sending a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge,
}

/// Represents errors that can occur when replying to a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyError {
    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge,

    /// The receiver is not expecting a reply.
    NotWaitingForReply,
}

/// Initializes the IPC subsystem by setting up the necessary data structures.
pub fn setup() {
    PENDING_MESSAGE_TASKS.call_once(|| spin::Mutex::new(HashMap::new()));
    RECEIVERS_QUEUE.call_once(|| spin::Mutex::new(HashMap::new()));
    MESSAGES.call_once(|| spin::Mutex::new(HashMap::new()));
}

/// Sends a message from one process to another and waits until a reply is
/// received. The function is asynchronous and yields control while waiting
/// for the reply.
///
/// # Errors
/// Returns a [`SendError`] if the message could not be sent. Possible errors
/// include:
/// - [`SendError::PayloadTooLarge`]: The payload size exceeds the maximum
///   allowed size. See [`Message::MAX_PAYLOAD_SIZE`] for the limit.
///
/// # Panics
/// Panics if the IPC subsystem has not been initialized.
pub async fn send(
    from: usize,
    to: usize,
    operation: usize,
    payload: &[u8],
) -> Result<Message, SendError> {
    if payload.len() > Message::MAX_PAYLOAD_SIZE {
        return Err(SendError::PayloadTooLarge);
    }

    // Create the message to be sent
    let message = Message {
        sender: from,
        receiver: to,
        operation,
        payload_len: payload.len(),
        sender_queue: future::wait::Queue::new(),
        payload: {
            let mut buf = [0; Message::MAX_PAYLOAD_SIZE];
            buf[..payload.len()].copy_from_slice(payload);
            buf
        },
    };

    // Clone the sender's queue to use it for waiting for the reply.
    let sender_queue = message.sender_queue.clone();

    // Insert the message into the MESSAGES table and record that the sender
    // is waiting for a reply from the receiver
    get_message_storage().lock().insert(from, message);
    get_pending_tasks_storage()
        .lock()
        .entry(to)
        .or_default()
        .push(from);

    // Wake up the receiver if it is waiting for messages
    if let Some(queue) = get_receivers_queue().lock().remove(&to) {
        queue.wake_all();
    }

    // Wait for a reply by yielding until a message is available for the sender
    loop {
        // Check if there is a reply message for the sender.
        {
            let mut messages = get_message_storage().lock();
            if let Some(reply) = messages.get(&from)
                && reply.sender == to
            {
                return Ok(messages.remove(&from).unwrap());
            }
        }

        // Sleep until woken up by the receiver when a reply is available.
        future::wait::wait(&sender_queue).await;
    }
}

/// Receives a message for the specified receiver process. The function is
/// asynchronous and yields control while waiting for a message to arrive.
///
/// # Panics
/// Panics if the IPC subsystem has not been initialized.
pub async fn receive(to: usize) -> Message {
    loop {
        // Check if there is a message for the receiver
        if let Some(from) = get_pending_tasks_storage()
            .lock()
            .get_mut(&to)
            .and_then(alloc::vec::Vec::pop)
        {
            // There is a message for the receiver. Duplicate it and return it.
            // Why clone? Because we need to keep the message in the MESSAGES table
            // until the reply is sent back to the sender.
            if let Some(message) = get_message_storage().lock().get(&from) {
                return message.clone();
            }

            // This should not happen, as we have a pending task for this
            // receiver, but no message. Log an error and continue waiting.
            log::error!(
                "Inconsistent state: pending task {:?} for receiver {:?} but no message",
                from,
                to
            );
        }

        // Sleep until woken up by a sender when a new message arrives.
        let queue = get_receivers_queue().lock().entry(to).or_default().clone();
        future::wait::wait(&queue).await;
    }
}

/// Sends a reply message from one process to another.
///
/// # Errors
/// Returns a `ReplyError` if the reply could not be sent. Possible errors
/// include:
/// - [`ReplyError::PayloadTooLarge`]: The payload size exceeds the maximum allowed size.
/// - [`ReplyError::NotWaitingForReply`]: The receiver is not expecting a reply from the
///   sender. This usually means that there was no prior message sent from the
///   sender to the receiver.
///
/// # Panics
/// Panics if the IPC subsystem has not been initialized.
pub fn reply(from: usize, to: usize, status: usize, payload: &[u8]) -> Result<(), ReplyError> {
    if payload.len() > Message::MAX_PAYLOAD_SIZE {
        return Err(ReplyError::PayloadTooLarge);
    }

    // Check if the receiver is waiting for a reply. We can know this by
    // checking if there is a message stored for the receiver in the MESSAGES
    // table. We ensure that the sender is indeed the one waiting for the reply.
    let mut messages = get_message_storage().lock();
    match messages.get(&to) {
        Some(msg) => {
            if msg.receiver != from {
                return Err(ReplyError::NotWaitingForReply);
            }
            msg.sender_queue.wake_all();
        }
        None => return Err(ReplyError::NotWaitingForReply),
    }

    // Remove the original message from the storage
    messages.remove(&to);

    // Create the reply message
    let mut message = Message {
        sender: from,
        receiver: to,
        operation: status,
        payload_len: payload.len(),
        payload: [0; Message::MAX_PAYLOAD_SIZE],
        sender_queue: future::wait::Queue::new(),
    };

    // Copy the payload into the message and insert it into the
    // MESSAGES table for the receiver to pick up later
    message.payload[..payload.len()].copy_from_slice(payload);
    messages.insert(to, message);
    Ok(())
}

/// An helper function to get a reference to the global MESSAGES storage.
///
/// # Panics
/// Panics if the IPC subsystem has not been initialized.
fn get_message_storage() -> &'static spin::Mutex<HashMap<usize, Message>> {
    MESSAGES.get().expect("MESSAGES not initialized")
}

/// An helper function to get a reference to the global `PENDING_MESSAGE_TASKS`
/// storage.
///
/// # Panics
/// Panics if the IPC subsystem has not been initialized.
fn get_pending_tasks_storage() -> &'static spin::Mutex<HashMap<usize, Vec<usize>>> {
    PENDING_MESSAGE_TASKS
        .get()
        .expect("PENDING_MESSAGE_TASKS not initialized")
}

/// An helper function to get a reference to the global `RECEIVERS_QUEUE`
/// storage.
///
/// # Panics
/// Panics if the IPC subsystem has not been initialized.
fn get_receivers_queue() -> &'static spin::Mutex<HashMap<usize, future::wait::Queue>> {
    RECEIVERS_QUEUE
        .get()
        .expect("RECEIVER_QUEUE not initialized")
}
