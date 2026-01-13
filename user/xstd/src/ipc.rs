use core::mem::MaybeUninit;

use crate::syscall::{self, SyscallCode};

impl SyscallCode for ::syscall::ipc::SendError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => ::syscall::ipc::SendError::InvalidDestination,
            2 => ::syscall::ipc::SendError::BadMessage,
            3 => ::syscall::ipc::SendError::PayloadTooLarge,
            _ => ::syscall::ipc::SendError::Unknown,
        }
    }
}

impl SyscallCode for ::syscall::ipc::ReceiveError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => ::syscall::ipc::ReceiveError::BadBuffer,
            _ => ::syscall::ipc::ReceiveError::Unknown,
        }
    }
}

impl SyscallCode for ::syscall::ipc::ReplyError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => ::syscall::ipc::ReplyError::InvalidDestination,
            2 => ::syscall::ipc::ReplyError::BadMessage,
            3 => ::syscall::ipc::ReplyError::PayloadTooLarge,
            4 => ::syscall::ipc::ReplyError::NotWaitingForReply,
            _ => ::syscall::ipc::ReplyError::Unknown,
        }
    }
}

/// Sends an IPC message to the specified receiver task ID, and blocks until
/// until a reply is received.
///
/// # Errors
/// Returns an [`IpcSendError`] describing the error if the syscall fails.
pub fn send(
    receiver: usize,
    kind: usize,
    payload: &[u8],
) -> Result<::syscall::ipc::Reply, ::syscall::ipc::SendError> {
    let mut message = ::syscall::ipc::Message {
        sender: 0,
        receiver,
        kind,
        payload_len: payload.len(),
        payload: [0u8; ::syscall::ipc::MAX_PAYLOAD_SIZE],
    };
    let mut reply = MaybeUninit::<::syscall::ipc::Reply>::uninit();
    let ret;

    message.payload[..payload.len().min(::syscall::ipc::MAX_PAYLOAD_SIZE)]
        .copy_from_slice(&payload[..payload.len().min(::syscall::ipc::MAX_PAYLOAD_SIZE)]);

    unsafe {
        core::arch::asm!("ecall",
            in("a7") 6,             // syscall number for ipc_send
            in("a0") &message,      // pointer to the message
            in("a1") &mut reply,    // pointer to the reply
            lateout("a0") ret,      // return value
            options(nostack, preserves_flags)
        );
    }

    if syscall::failed(ret) {
        Err(::syscall::ipc::SendError::from_syscall_code(ret as isize))
    } else {
        // SAFETY: The syscall succeeded, so the reply should be properly
        // initialized by the kernel. If we can't trust the kernel, we are
        // already in trouble !
        Ok(unsafe { reply.assume_init() })
    }
}

/// Receives an IPC message sent to the current task, blocking until a message
/// is available.
///
/// # Errors
/// Returns an [`ReceiveError`] describing the error if the syscall fails.
pub fn receive() -> Result<::syscall::ipc::Message, ::syscall::ipc::ReceiveError> {
    let mut message = MaybeUninit::<::syscall::ipc::Message>::uninit();
    let ret;

    unsafe {
        core::arch::asm!("ecall",
            in("a7") 7,                     // syscall number for ipc_receive
            in("a0") &mut message,          // pointer to the message buffer
            lateout("a0") ret,              // return value
            options(nostack, preserves_flags)
        );
    }

    if syscall::failed(ret) {
        Err(::syscall::ipc::ReceiveError::from_syscall_code(
            ret as isize,
        ))
    } else {
        // SAFETY: The syscall succeeded, so the message should be properly
        // initialized by the kernel.
        Ok(unsafe { message.assume_init() })
    }
}

/// Replies to an IPC message sent from another task.
///
/// # Errors
/// Returns an [`IpcReplyError`] describing the error if the syscall fails.
/// Most notably, this can happen if the destination task is not waiting for
/// a reply (meaning it did not send a message to this task).
pub fn reply(to: usize, status: usize, payload: &[u8]) -> Result<(), ::syscall::ipc::ReplyError> {
    let mut reply = ::syscall::ipc::Reply {
        status,
        payload_len: payload.len(),
        payload: [0u8; ::syscall::ipc::MAX_PAYLOAD_SIZE],
    };
    let ret;

    reply.payload[..payload.len().min(::syscall::ipc::MAX_PAYLOAD_SIZE)]
        .copy_from_slice(&payload[..payload.len().min(::syscall::ipc::MAX_PAYLOAD_SIZE)]);

    unsafe {
        core::arch::asm!("ecall",
            in("a7") 8,                 // syscall number for ipc_reply
            in("a0") to,                // destination task ID
            in("a1") &reply,            // pointer to the reply
            lateout("a0") ret,          // return value
            options(nostack, preserves_flags)
        );
    }

    if syscall::failed(ret) {
        Err(::syscall::ipc::ReplyError::from_syscall_code(ret as isize))
    } else {
        Ok(())
    }
}
