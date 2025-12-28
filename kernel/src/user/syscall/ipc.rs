use zerocopy::{FromBytes, IntoBytes};

use crate::{
    arch::{thread::Thread, trap::Resume},
    future, ipc,
    user::{object::Object, ptr::Pointer, syscall::SyscallReturnValue},
};

/// Maximum payload size for IPC messages.
const MAX_PAYLOAD_SIZE: usize = 256;

/// Represents an IPC message used by syscalls to reduce the number of
/// parameters passed. We use the C representation to ensure a predictable
/// layout compatible with the kernel.
#[derive(FromBytes, IntoBytes)]
#[repr(C)]
pub struct Message {
    /// The sender task ID. If the message is sent from user space, this
    /// field is ignored and will be filled in by the kernel.
    pub sender: usize,

    /// The receiver task ID. If the message is sent to user space, this
    /// field is ignored and will be filled in by the kernel.
    pub receiver: usize,

    /// The message kind.
    pub kind: usize,

    /// The length of the payload.
    pub payload_len: usize,

    /// The payload data.
    pub payload: [u8; MAX_PAYLOAD_SIZE],
}

/// Represents an IPC reply used by syscalls to reduce the number of
/// parameters passed. We use the C representation to ensure a predictable
/// layout compatible with the kernel.
#[derive(FromBytes, IntoBytes)]
#[repr(C)]
pub struct Reply {
    /// The status of the reply.
    pub status: usize,

    /// The length of the payload.
    pub payload_len: usize,

    /// The payload data.
    pub payload: [u8; MAX_PAYLOAD_SIZE],
}

/// Errors that can occur when sending an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcSendError {
    /// The destination is invalid.
    InvalidDestination = 1,

    /// The message is invalid.
    BadMessage = 2,

    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge = 3,
}

impl From<ipc::message::SendError> for IpcSendError {
    fn from(error: ipc::message::SendError) -> Self {
        match error {
            ipc::message::SendError::PayloadTooLarge => IpcSendError::PayloadTooLarge,
        }
    }
}

impl From<IpcSendError> for isize {
    fn from(error: IpcSendError) -> Self {
        match error {
            IpcSendError::InvalidDestination => 1,
            IpcSendError::BadMessage => 2,
            IpcSendError::PayloadTooLarge => 3,
        }
    }
}

/// Errors that can occur when receiving an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcReceiveError {
    /// The buffer pointer is invalid.
    BadBuffer = 1,
}

impl From<IpcReceiveError> for isize {
    fn from(error: IpcReceiveError) -> Self {
        match error {
            IpcReceiveError::BadBuffer => 1,
        }
    }
}

/// Errors that can occur when replying to an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcReplyError {
    /// The destination is invalid.
    InvalidDestination = 1,

    /// The message is invalid.
    BadMessage = 2,

    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge = 3,

    /// The task is not waiting for a reply from the sender.
    NotWaitingForReply = 4,
}

impl From<ipc::message::ReplyError> for IpcReplyError {
    fn from(error: ipc::message::ReplyError) -> Self {
        match error {
            ipc::message::ReplyError::PayloadTooLarge => IpcReplyError::PayloadTooLarge,
            ipc::message::ReplyError::NotWaitingForReply => IpcReplyError::NotWaitingForReply,
        }
    }
}

impl From<IpcReplyError> for isize {
    fn from(error: IpcReplyError) -> Self {
        match error {
            IpcReplyError::InvalidDestination => 1,
            IpcReplyError::BadMessage => 2,
            IpcReplyError::PayloadTooLarge => 3,
            IpcReplyError::NotWaitingForReply => 4,
        }
    }
}

/// Sends an IPC message from the current task to another task and waits
/// for a reply.
///
/// # Parameters
/// - `thread`: The current thread context.
/// - `message_ptr`: An user pointer to the message to be sent
/// - `reply_ptr`: An user pointer to where the reply should be written.
///
/// # Errors
/// If the syscall fails, an appropriate [`IpcSendError`] is returned
/// describing the failure reason.
///
/// # Panics
/// This function may panic if the current task ID cannot be retrieved. This
/// should never happen since this function is called from a task context.
pub async fn send(
    thread: &mut Thread,
    message_ptr: Pointer<Message>,
    reply_ptr: Pointer<Reply>,
) -> Result<SyscallReturnValue, IpcSendError> {
    // Read the message from user space and get the current task ID.
    let message = unsafe { Object::<Message>::new(message_ptr) };
    let id = future::executor::current_task_id().unwrap();

    // Validate the payload size, ensuring it does not exceed the maximum
    // allowed size to avoid buffer overflows.
    if message.payload_len > MAX_PAYLOAD_SIZE {
        return Err(IpcSendError::PayloadTooLarge);
    }

    // Send the message and wait for the reply.
    let reply = ipc::message::send(
        usize::from(id),
        message.receiver,
        message.kind,
        &message.payload[..message.payload_len],
    )
    .await?;

    // Construct the reply to be sent back to user space.
    let reply = Reply {
        status: reply.operation,
        payload_len: reply.payload_len,
        payload: {
            let mut payload = [0u8; MAX_PAYLOAD_SIZE];
            payload[..reply.payload_len].copy_from_slice(&reply.payload[..reply.payload_len]);
            payload
        },
    };

    // Write the reply back to user space.
    // SAFETY: This is safe because we have verified that the pointer is valid
    // when creating the `Pointer<Reply>` in the syscall handler, and we
    // ensure that we are writing to the correct user address space by setting
    // the root table of the current thread as current before writing.
    unsafe {
        thread.root_table().set_current();
        Object::write(&reply_ptr, &reply);
    }

    Ok(SyscallReturnValue {
        resume: Resume::Continue,
        value: 0,
    })
}

/// Receives an IPC message for the current task. If no message is available,
/// the function will yield until a message arrives.
///
/// # Parameters
/// - `thread`: The current thread context.
/// - `message_ptr`: An user pointer to where the received message should
///   be written.
///
/// # Errors
/// If the syscall fails, an appropriate [`IpcReceiveError`] is returned
/// describing the failure reason.
///
/// # Panics
/// This function may panic if the current task ID cannot be retrieved. This
/// should never happen since this function is called from a task context.
pub async fn receive(
    thread: &mut Thread,
    message_ptr: Pointer<Message>,
) -> Result<SyscallReturnValue, IpcReceiveError> {
    let id = future::executor::current_task_id().unwrap();
    let received = ipc::message::receive(usize::from(id)).await;

    // Construct the message to be sent back to user space.
    let message = Message {
        sender: received.sender,
        receiver: received.receiver,
        kind: received.operation,
        payload_len: received.payload_len,
        payload: {
            let mut payload = [0u8; MAX_PAYLOAD_SIZE];
            payload[..received.payload_len]
                .copy_from_slice(&received.payload[..received.payload_len]);
            payload
        },
    };

    // Write the message back to user space.
    // SAFETY: This is safe because we have verified that the pointer is valid
    // when creating the `Pointer<Message>` in the syscall handler, and we
    // ensure that we are writing to the correct user address space by setting
    // the root table of the current thread as current before writing.
    unsafe {
        thread.root_table().set_current();
        Object::write(&message_ptr, &message);
    }

    Ok(SyscallReturnValue {
        resume: Resume::Continue,
        value: 0,
    })
}

/// Replies to an IPC message from another task.
///
/// # Parameters
/// - `to`: The task ID of the task to reply to.
/// - `reply`: An user pointer to the reply message.
///
/// # Errors
/// If the syscall fails, an appropriate [`IpcReplyError`] is returned
/// describing the failure reason.
///
/// # Panics
/// This function may panic if the current task ID cannot be retrieved. This
/// should never happen since this function is called from a task context.
pub fn reply(to: usize, reply: Pointer<Reply>) -> Result<SyscallReturnValue, IpcReplyError> {
    // Read the reply from user space and get the current task ID.
    let reply = unsafe { Object::<Reply>::new(reply) };
    let id = future::executor::current_task_id().unwrap();

    // Reply to the message. This is a synchronous operation that is guaranteed
    // to complete immediately since the task being replied to is waiting for
    // the reply. If the task is not waiting for a reply, an error is returned.
    ipc::message::reply(
        usize::from(id),
        to,
        reply.status,
        &reply.payload[..reply.payload_len],
    )?;

    Ok(SyscallReturnValue {
        resume: Resume::Continue,
        value: 0,
    })
}
