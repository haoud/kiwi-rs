use zerocopy::{FromBytes, IntoBytes};

/// Maximum payload size for IPC messages.
pub const MAX_PAYLOAD_SIZE: usize = 256;

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
pub enum SendError {
    /// An unknown error occurred.
    Unknown = 0,

    /// The destination is invalid.
    InvalidDestination = 1,

    /// The message is invalid.
    BadMessage = 2,

    /// The payload size exceeds the maximum allowed size.
    PayloadTooLarge = 3,

    /// The target task does not exist.
    TaskDoesNotExist = 4,

    /// The target task has been destroyed before the message could be sent.
    TaskDestroyed = 5,
}

impl From<SendError> for isize {
    fn from(error: SendError) -> Self {
        match error {
            SendError::Unknown => 0,
            SendError::InvalidDestination => 1,
            SendError::BadMessage => 2,
            SendError::PayloadTooLarge => 3,
            SendError::TaskDoesNotExist => 4,
            SendError::TaskDestroyed => 5,
        }
    }
}

/// Errors that can occur when receiving an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveError {
    /// An unknown error occurred.
    Unknown = 0,

    /// The buffer pointer is invalid.
    BadBuffer = 1,
}

impl From<ReceiveError> for isize {
    fn from(error: ReceiveError) -> Self {
        match error {
            ReceiveError::Unknown => 0,
            ReceiveError::BadBuffer => 1,
        }
    }
}

/// Errors that can occur when replying to an IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyError {
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

    /// The receiver expected a reply from a different sender.
    UnexpectedSender = 5,

    /// The target task does not exist.
    TaskDoesNotExist = 6,

    /// The target task has been destroyed before the reply could be sent.
    TaskDestroyed = 7,
}

impl From<ReplyError> for isize {
    fn from(error: ReplyError) -> Self {
        match error {
            ReplyError::Unknown => 0,
            ReplyError::InvalidDestination => 1,
            ReplyError::BadMessage => 2,
            ReplyError::PayloadTooLarge => 3,
            ReplyError::NotWaitingForReply => 4,
            ReplyError::UnexpectedSender => 5,
            ReplyError::TaskDoesNotExist => 6,
            ReplyError::TaskDestroyed => 7,
        }
    }
}
