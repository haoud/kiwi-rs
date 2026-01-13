use crate::{
    arch::{thread::Thread, trap::Resume},
    future, ipc,
    user::{object::Object, ptr::Pointer, syscall::SyscallReturnValue},
};

impl From<ipc::message::SendError> for syscall::ipc::SendError {
    fn from(error: ipc::message::SendError) -> Self {
        match error {
            ipc::message::SendError::PayloadTooLarge => syscall::ipc::SendError::PayloadTooLarge,
        }
    }
}

impl From<ipc::message::ReplyError> for syscall::ipc::ReplyError {
    fn from(error: ipc::message::ReplyError) -> Self {
        match error {
            ipc::message::ReplyError::PayloadTooLarge => syscall::ipc::ReplyError::PayloadTooLarge,
            ipc::message::ReplyError::NotWaitingForReply => {
                syscall::ipc::ReplyError::NotWaitingForReply
            }
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
    message_ptr: Pointer<syscall::ipc::Message>,
    reply_ptr: Pointer<syscall::ipc::Reply>,
) -> Result<SyscallReturnValue, syscall::ipc::SendError> {
    // Read the message from user space and get the current task ID.
    let message = unsafe { Object::<syscall::ipc::Message>::new(message_ptr) };
    let id = future::executor::current_task_id().unwrap();

    // Validate the payload size, ensuring it does not exceed the maximum
    // allowed size to avoid buffer overflows.
    if message.payload_len > syscall::ipc::MAX_PAYLOAD_SIZE {
        return Err(syscall::ipc::SendError::PayloadTooLarge);
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
    let reply = syscall::ipc::Reply {
        status: reply.operation,
        payload_len: reply.payload_len,
        payload: {
            let mut payload = [0u8; syscall::ipc::MAX_PAYLOAD_SIZE];
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
    message_ptr: Pointer<syscall::ipc::Message>,
) -> Result<SyscallReturnValue, syscall::ipc::ReceiveError> {
    let id = future::executor::current_task_id().unwrap();
    let received = ipc::message::receive(usize::from(id)).await;

    // Construct the message to be sent back to user space.
    let message = syscall::ipc::Message {
        sender: received.sender,
        receiver: received.receiver,
        kind: received.operation,
        payload_len: received.payload_len,
        payload: {
            let mut payload = [0u8; syscall::ipc::MAX_PAYLOAD_SIZE];
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
pub fn reply(
    to: usize,
    reply: Pointer<syscall::ipc::Reply>,
) -> Result<SyscallReturnValue, syscall::ipc::ReplyError> {
    // Read the reply from user space and get the current task ID.
    let reply = unsafe { Object::<syscall::ipc::Reply>::new(reply) };
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
