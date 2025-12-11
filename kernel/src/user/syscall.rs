use crate::arch::{self, trap::Resume};

/// Enumeration of supported syscall operations by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SyscallOp {
    /// No operation syscall, used for testing purposes.
    Nop = 0,

    /// Exit the current task.
    Exit = 1,

    /// Used for any unknown syscall IDs.
    Unknown = u32::MAX,
}

impl From<usize> for SyscallOp {
    fn from(value: usize) -> Self {
        match u32::try_from(value).unwrap_or(u32::MAX) {
            0 => SyscallOp::Nop,
            1 => SyscallOp::Exit,
            _ => SyscallOp::Unknown,
        }
    }
}

/// Handles a syscall invoked by the given thread.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
pub fn handle_syscall(thread: &mut arch::thread::Thread) -> Resume {
    let args = arch::thread::get_syscall_args(thread);
    let id = arch::thread::get_syscall_id(thread);

    log::trace!("Handling syscall ID: {}", id);
    match SyscallOp::from(id) {
        SyscallOp::Nop => Resume::Continue,
        SyscallOp::Exit => Resume::Terminate(args[0] as i32),
        SyscallOp::Unknown => {
            log::warn!("Unknown syscall ID: {}", id);
            Resume::Continue
        }
    }
}
