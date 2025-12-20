use crate::{
    arch::{self, trap::Resume},
    user::syscall,
};

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
            _ => SyscallOp::Unknown,
        }
    }
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
pub fn handle_syscall(thread: &mut arch::thread::Thread) -> Resume {
    let args = arch::thread::get_syscall_args(thread);
    let id = arch::thread::get_syscall_id(thread);

    log::trace!("Handling syscall ID: {}", id);
    let result = match SyscallOp::from(id) {
        SyscallOp::Nop => Ok(Resume::Continue),
        SyscallOp::TaskExit => Ok(Resume::Terminate(args[0] as i32)),
        SyscallOp::TaskYield => Ok(Resume::Yield),
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
        SyscallOp::Unknown => {
            log::warn!("Unknown syscall ID: {}", id);
            Ok(Resume::Continue)
        }
    };

    match result {
        Ok(resume) => {
            log::trace!("Syscall completed successfully.");
            arch::thread::set_syscall_return(thread, 0);
            resume
        }
        Err(e) => {
            log::trace!("Syscall failed with error code: {}", e);
            arch::thread::set_syscall_return(thread, -e);
            Resume::Continue
        }
    }
}
