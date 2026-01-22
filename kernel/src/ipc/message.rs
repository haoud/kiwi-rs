use alloc::boxed::Box;

use crate::future::{self};

/// Represents a message sent between tasks.
#[derive(Debug, Clone)]
pub struct Message {
    /// The sender's task identifier.
    pub sender: future::task::Identifier,

    /// The receiver's task identifier.
    pub receiver: future::task::Identifier,

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

/// Represents the IPC waiting state of a task. This enum defines the
/// different states a task can be when waiting for IPC operations.
#[derive(Debug)]
pub enum IpcWaitingState {
    /// The task does not wait for anything.
    None,

    /// The task is waiting to send a message.
    WaitingForSend,

    /// The task is waiting to receive a message.
    WaitingForMessage,

    /// The task is waiting for a reply to a previously sent message by
    /// the specified task identifier.
    WaitingForReply(future::task::Identifier),
}

impl IpcWaitingState {
    /// Sets the IPC state to `WaitingForReply`.
    pub fn set_waiting_for_reply(&mut self, from: future::task::Identifier) {
        *self = IpcWaitingState::WaitingForReply(from);
    }

    /// Sets the IPC state to `WaitingForMessage`
    pub fn set_waiting_for_message(&mut self) {
        *self = IpcWaitingState::WaitingForMessage;
    }

    /// Sets the IPC state to `WaitingForSend`.
    pub fn set_waiting_for_send(&mut self) {
        *self = IpcWaitingState::WaitingForSend;
    }
}

/// Represents errors that can occur when sending a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge,

    /// The target task does not exist.
    TaskDoesNotExist,

    /// The target task has been destroyed before the message could be sent.
    TaskDestroyed,
}

/// Represents errors that can occur when replying to a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyError {
    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge,

    /// The receiver is not expecting a reply.
    NotWaitingForReply,

    /// The receiver expected a reply from a different sender.
    UnexpectedSender,

    /// The target task does not exist.
    TaskDoesNotExist,

    /// The target task has been destroyed before the reply could be sent.
    TaskDestroyed,
}

/// Sends a message from one process to another and waits until a reply is
/// received.
///
/// # Errors
/// Returns a [`SendError`] if the message could not be sent or if the reply
/// could not be received (mostly due to the target task being destroyed
/// before the operation could complete).
///
/// # Panics
/// Panics if there is no current task context. This can only happen if this
/// function is called during kernel initialization, before any tasks have been
/// created, and is a serious programming error.
pub async fn send(
    to: future::task::Identifier,
    operation: usize,
    payload: &[u8],
) -> Result<Box<Message>, SendError> {
    if payload.len() > Message::MAX_PAYLOAD_SIZE {
        return Err(SendError::PayloadTooLarge);
    }

    // Check that the target task exists
    if !future::task::exists(to) {
        return Err(SendError::TaskDoesNotExist);
    }

    // Create the message to be sent
    let from = future::executor::current_task_id().unwrap();
    let message = Box::new(Message {
        sender: from,
        receiver: to,
        operation,
        payload_len: payload.len(),
        payload: {
            let mut buf = [0; Message::MAX_PAYLOAD_SIZE];
            buf[..payload.len()].copy_from_slice(payload);
            buf
        },
    });

    // Send the message by changing the IPC state of the receiver process
    // if it is waiting for messages. Otherwise, we change our own state to
    // waiting for a reply, and wait until the receiver is ready to process it.
    loop {
        let send_queue = future::task::try_with_local_set_from(to, |set| {
            if let Some(receiver_local_set) = set {
                match &*receiver_local_set.ipc_waiting_state.lock() {
                    IpcWaitingState::WaitingForMessage => {
                        // The receiver is waiting for messages. Due to
                        // borrowing rules, we cannot set the message directly
                        // here since the compiler does not know that this
                        // will be the last iteration before we break out of
                        // the loop, and throws a borrow error. So we return
                        // None to indicate that we can proceed to deliver
                        // the message.
                        Ok(None)
                    }
                    _ => Ok(Some(receiver_local_set.ipc_send_queue.clone())),
                }
            } else {
                // The target task has been destroyed before we could
                // send the message. Return an error to the caller.
                Err(SendError::TaskDestroyed)
            }
        })?;

        if let Some(queue) = send_queue {
            // The receiver was not waiting for messages. We need to wait
            // until it is ready to receive our message. Set our IPC state
            // to waiting for send and wait on the associated queue.
            future::task::with_current_local_set(|current_local_set| {
                current_local_set
                    .ipc_waiting_state
                    .lock()
                    .set_waiting_for_send();
            });
            future::wait::wait(&queue).await;
        } else {
            future::task::try_with_local_set_from(to, |set| {
                if let Some(receiver_local_set) = set {
                    // Wake up the receiver since it is waiting for messages,
                    // and deliver the message to the receiver's local data
                    // set.
                    receiver_local_set.ipc_message.lock().replace(message);
                    receiver_local_set.ipc_receive_queue.wake_one();
                    Ok(())
                } else {
                    // The target task has been destroyed before we could
                    // send the message. Return an error to the caller.
                    Err(SendError::TaskDestroyed)
                }
            })?;
            break;
        }
    }

    // Now that the message has been sent, wait for the reply. Set our IPC
    // state to waiting for reply and wait on the associated queue.
    loop {
        let reply = future::task::with_current_local_set(|current_local_set| {
            if let Some(reply) = current_local_set.ipc_reply.lock().take() {
                // A reply has been received. Return it.
                Ok(Some(reply))
            } else if !future::task::exists(to) {
                // The target task has been destroyed before sending
                // the reply. Return an error to the caller.
                Err(SendError::TaskDestroyed)
            } else {
                // No reply yet. Set the state to waiting for reply from the
                // receiver process, and return the associated reply queue
                // to wait on and be woken up when the reply arrives.
                current_local_set
                    .ipc_waiting_state
                    .lock()
                    .set_waiting_for_reply(to);
                Ok(None)
            }
        })?;

        if let Some(reply) = reply {
            break Ok(reply);
        }

        // We are still waiting for the reply. Sleep and wait to be woken up
        // when the reply arrives, or when the target task is destroyed.
        let queue = future::task::try_with_local_set_from(to, |receiver_local_set| {
            if let Some(set) = receiver_local_set {
                Ok(set.ipc_reply_queue.clone())
            } else {
                // The target task has been destroyed before sending
                // the reply. Return an error to the caller.
                Err(SendError::TaskDestroyed)
            }
        })?;
        future::wait::wait(&queue).await;
    }
}

/// Receives a message for the specified receiver process. The function is
/// asynchronous and yields control while waiting for a message to arrive.
///
/// # Panics
/// Panics if there is no current task context. This can only happen if this
/// function is called during kernel initialization, before any tasks have been
/// created, and is a serious programming error.
pub async fn receive() -> Box<Message> {
    loop {
        // Check if there is a message for the receiver.
        let message = future::task::with_current_local_set(|current_local_set| {
            current_local_set.ipc_message.lock().take()
        });

        // Yes, a message is available. Return it.
        if let Some(message) = message {
            break message;
        }

        // No message available yet. Change the IPC state to indicate that we
        // are waiting for a message, wake up any senders waiting to send us
        // messages, and wait on our receive queue to be woken up when a
        // message arrives.
        let queue = future::task::with_current_local_set(|local_set| {
            local_set.ipc_waiting_state.lock().set_waiting_for_message();
            local_set.ipc_send_queue.wake_all();
            local_set.ipc_receive_queue.clone()
        });
        future::wait::wait(&queue).await;
    }
}

/// Sends a reply message from one process to another.
///
/// # Errors
/// Returns a [`ReplyError`] if the reply could not be sent.
///
/// # Panics
/// Panics if there is no current task context. This can only happen if this
/// function is called during kernel initialization, before any tasks have been
/// created, and is a serious programming error.
pub fn reply(
    to: future::task::Identifier,
    status: usize,
    payload: &[u8],
) -> Result<(), ReplyError> {
    if payload.len() > Message::MAX_PAYLOAD_SIZE {
        return Err(ReplyError::PayloadTooLarge);
    }

    if !future::task::exists(to) {
        return Err(ReplyError::TaskDoesNotExist);
    }

    // Create the reply message
    let from = future::executor::current_task_id().unwrap();
    let message = Box::new(Message {
        sender: from,
        receiver: to,
        operation: status,
        payload_len: payload.len(),
        payload: {
            let mut buf = [0; Message::MAX_PAYLOAD_SIZE];
            buf[..payload.len()].copy_from_slice(payload);
            buf
        },
    });

    // Check if the receiver is waiting for a reply by checking its IPC state,
    // and ensure that it is waiting for a reply from the correct sender. If
    // so, deliver the reply message and wake up the receiver.
    future::task::try_with_local_set_from(to, |set| {
        if let Some(receiver_local_set) = set {
            match *receiver_local_set.ipc_waiting_state.lock() {
                IpcWaitingState::WaitingForReply(expected_from) => {
                    if expected_from != from {
                        return Err(ReplyError::UnexpectedSender);
                    }
                    receiver_local_set.ipc_reply.lock().replace(message);
                    Ok(())
                }
                _ => Err(ReplyError::NotWaitingForReply),
            }
        } else {
            // The target task has been destroyed before we could
            // send the reply. Return an error to the caller.
            Err(ReplyError::TaskDestroyed)
        }
    })?;

    // Since we can't know which task will be woken up (in case of multiple
    // tasks waiting for a reply), we wake them all up. This can happen if
    // the task handle multiple IPC receives before replying to any of them.
    // TODO: Only wake up the task that we replied to.
    future::task::with_current_local_set(|current_local_set| {
        current_local_set.ipc_reply_queue.wake_all();
    });

    Ok(())
}
