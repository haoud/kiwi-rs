use core::mem::MaybeUninit;

use crate::syscall::{self, SyscallCode};

/// Maximum payload size for IPC messages.
const MAX_PAYLOAD_SIZE: usize = 256;

/// Represents an IPC message used by syscalls to reduce the number of
/// parameters passed. We use the C representation to ensure a predictable
/// layout compatible with the kernel.
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
    /// An unknown error occurred.
    Unknown = 0,

    /// The destination is invalid.
    InvalidDestination = 1,

    /// The message is invalid.
    BadMessage = 2,

    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge = 3,
}

impl SyscallCode for IpcSendError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => IpcSendError::InvalidDestination,
            2 => IpcSendError::BadMessage,
            3 => IpcSendError::PayloadTooLarge,
            _ => IpcSendError::Unknown,
        }
    }
}

/// Errors that can occur when receiving an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcReceiveError {
    /// An unknown error occurred.
    Unknown = 0,

    /// The buffer pointer is invalid.
    BadBuffer = 1,
}

impl SyscallCode for IpcReceiveError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => IpcReceiveError::BadBuffer,
            _ => IpcReceiveError::Unknown,
        }
    }
}

/// Errors that can occur when replying to an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcReplyError {
    /// An unknown error occurred.
    Unknown = 0,

    /// The destination is invalid.
    InvalidDestination = 1,

    /// The message is invalid.
    BadMessage = 2,

    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge = 3,

    /// The task is not waiting for a reply from the sender.
    NotWaitingForReply = 4,
}

impl SyscallCode for IpcReplyError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => IpcReplyError::InvalidDestination,
            2 => IpcReplyError::BadMessage,
            3 => IpcReplyError::PayloadTooLarge,
            4 => IpcReplyError::NotWaitingForReply,
            _ => IpcReplyError::Unknown,
        }
    }
}

/// Sends an IPC message to the specified receiver task ID, and blocks until
/// until a reply is received.
///
/// # Errors
/// Returns an [`IpcSendError`] describing the error if the syscall fails.
pub fn send(receiver: usize, kind: usize, payload: &[u8]) -> Result<Reply, IpcSendError> {
    let mut message = Message {
        sender: 0,
        receiver,
        kind,
        payload_len: payload.len(),
        payload: [0u8; MAX_PAYLOAD_SIZE],
    };
    let mut reply = MaybeUninit::<Reply>::uninit();
    let ret;

    message.payload[..payload.len().min(MAX_PAYLOAD_SIZE)]
        .copy_from_slice(&payload[..payload.len().min(MAX_PAYLOAD_SIZE)]);

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
        Err(IpcSendError::from_syscall_code(ret as isize))
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
/// Returns an [`IpcReceiveError`] describing the error if the syscall fails.
pub fn receive() -> Result<Message, IpcReceiveError> {
    let mut message = MaybeUninit::<Message>::uninit();
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
        Err(IpcReceiveError::from_syscall_code(ret as isize))
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
pub fn reply(to: usize, status: usize, payload: &[u8]) -> Result<(), IpcReplyError> {
    let mut reply = Reply {
        status,
        payload_len: payload.len(),
        payload: [0u8; MAX_PAYLOAD_SIZE],
    };
    let ret;

    reply.payload[..payload.len().min(MAX_PAYLOAD_SIZE)]
        .copy_from_slice(&payload[..payload.len().min(MAX_PAYLOAD_SIZE)]);

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
        Err(IpcReplyError::from_syscall_code(ret as isize))
    } else {
        Ok(())
    }
}
