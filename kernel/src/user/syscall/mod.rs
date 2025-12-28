use crate::{
    arch::{self, trap::Resume},
    user::{ptr::Pointer, syscall},
};

pub mod ipc;
pub mod service;

/// Enumeration of supported syscall operations by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SyscallOp {
    /// No operation syscall, used for testing purposes.
    Nop = 0,

    /// Exit the current task.
    TaskExit = 1,

    /// Yield the current task's execution.
    TaskYield = 2,

    /// Register a new service.
    ServiceRegister = 3,

    /// Unregister a service.
    ServiceUnregister = 4,

    /// Connect to a service.
    ServiceConnect = 5,

    /// Send an IPC message
    IpcSend = 6,

    /// Receive an IPC message
    IpcReceive = 7,

    /// Reply to an IPC message
    IpcReply = 8,

    /// Used for any unknown syscall IDs.
    Unknown = u32::MAX,
}

impl From<usize> for SyscallOp {
    fn from(value: usize) -> Self {
        match u32::try_from(value).unwrap_or(u32::MAX) {
            0 => SyscallOp::Nop,
            1 => SyscallOp::TaskExit,
            2 => SyscallOp::TaskYield,
            3 => SyscallOp::ServiceRegister,
            4 => SyscallOp::ServiceUnregister,
            5 => SyscallOp::ServiceConnect,
            6 => SyscallOp::IpcSend,
            7 => SyscallOp::IpcReceive,
            8 => SyscallOp::IpcReply,
            _ => SyscallOp::Unknown,
        }
    }
}

/// Represents the return value of a syscall, including how the thread
/// should resume execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallReturnValue {
    pub resume: Resume,
    pub value: usize,
}

/// Handles a syscall invoked by the given thread.
///
/// # Panics
/// This function may panic if it encounters an unrecoverable error while
/// handling the syscall. This includes, but is not limited to:
/// - The executor does not have a current task when required (this should
///   never happen in normal operation).
#[must_use]
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
pub async fn handle_syscall(thread: &mut arch::thread::Thread) -> Resume {
    let args = arch::thread::get_syscall_args(thread);
    let id = arch::thread::get_syscall_id(thread);

    log::trace!("Handling syscall ID: {}", id);
    let result = match SyscallOp::from(id) {
        SyscallOp::Nop => Ok(SyscallReturnValue {
            resume: Resume::Continue,
            value: 0,
        }),
        SyscallOp::TaskExit => Ok(SyscallReturnValue {
            resume: Resume::Terminate(args[0] as i32),
            value: 0,
        }),
        SyscallOp::TaskYield => Ok(SyscallReturnValue {
            resume: Resume::Yield,
            value: 0,
        }),
        SyscallOp::ServiceRegister => {
            let name_ptr = core::ptr::with_exposed_provenance_mut::<u8>(args[0]);
            let name_len = args[1];
            syscall::service::register(name_ptr, name_len).map_err(isize::from)
        }
        SyscallOp::ServiceUnregister => {
            // Currently, no arguments are needed for unregistration since
            // the service is associated with the current task itself.
            syscall::service::unregister().map_err(isize::from)
        }
        SyscallOp::ServiceConnect => {
            let name_ptr = core::ptr::with_exposed_provenance_mut::<u8>(args[0]);
            let name_len = args[1];
            syscall::service::connect(name_ptr, name_len).map_err(isize::from)
        }
        SyscallOp::IpcSend => {
            let message_ptr = core::ptr::with_exposed_provenance::<syscall::ipc::Message>(args[0]);
            let reply_ptr = core::ptr::with_exposed_provenance_mut::<syscall::ipc::Reply>(args[1]);
            let message_ptr = Pointer::new(message_ptr.cast_mut())
                .ok_or(isize::from(syscall::ipc::IpcSendError::BadMessage));
            let reply_ptr =
                Pointer::new(reply_ptr).ok_or(isize::from(syscall::ipc::IpcSendError::BadMessage));

            if let (Ok(msg_ptr), Ok(rpl_ptr)) = (message_ptr, reply_ptr) {
                syscall::ipc::send(thread, msg_ptr, rpl_ptr)
                    .await
                    .map_err(isize::from)
            } else {
                Err(isize::from(syscall::ipc::IpcSendError::BadMessage))
            }
        }
        SyscallOp::IpcReceive => {
            let message_ptr =
                core::ptr::with_exposed_provenance_mut::<syscall::ipc::Message>(args[0]);
            let message_ptr = Pointer::new(message_ptr)
                .ok_or(isize::from(syscall::ipc::IpcReceiveError::BadBuffer));
            if let Ok(ptr) = message_ptr {
                syscall::ipc::receive(thread, ptr)
                    .await
                    .map_err(isize::from)
            } else {
                Err(isize::from(syscall::ipc::IpcReceiveError::BadBuffer))
            }
        }
        SyscallOp::IpcReply => {
            let to = args[0];
            let reply_ptr = core::ptr::with_exposed_provenance::<syscall::ipc::Reply>(args[1]);
            let reply_ptr = Pointer::new(reply_ptr.cast_mut())
                .ok_or(isize::from(syscall::ipc::IpcReplyError::BadMessage));
            if let Ok(ptr) = reply_ptr {
                syscall::ipc::reply(to, ptr).map_err(isize::from)
            } else {
                Err(isize::from(syscall::ipc::IpcReplyError::BadMessage))
            }
        }
        SyscallOp::Unknown => {
            log::warn!("Unknown syscall ID: {}", id);
            Ok(SyscallReturnValue {
                resume: Resume::Continue,
                value: usize::MAX,
            })
        }
    };

    match result {
        Ok(ret) => {
            log::trace!("Syscall completed successfully.");
            arch::thread::set_syscall_return(thread, ret.value as isize);
            ret.resume
        }
        Err(e) => {
            log::trace!("Syscall failed with error code: {}", e);
            arch::thread::set_syscall_return(thread, -e);
            Resume::Continue
        }
    }
}
