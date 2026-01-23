use crate::syscall::{self, SyscallCode};

impl SyscallCode for ::syscall::debug::WriteError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            0 => ::syscall::debug::WriteError::Unknown,
            1 => ::syscall::debug::WriteError::BadName,
            2 => ::syscall::debug::WriteError::NoOutputAvailable,
            _ => ::syscall::debug::WriteError::Unknown,
        }
    }
}

/// Writes a string to the kernel debug output. This is primarily intended
/// for debugging purposes, and may not be available in production builds.
///
/// # Errors
/// This function returns a [`WriteError`] if the write operation fails,
/// or the number of bytes written on success.
pub fn write(str: &str) -> Result<usize, ::syscall::debug::WriteError> {
    let ret;

    unsafe {
        core::arch::asm!("ecall",
            in("a7") 999,               // syscall number for debug_write
            in("a0") str.as_ptr(),      // pointer to the string
            in("a1") str.len(),         // length of the string
            lateout("a0") ret,          // return value
            options(nostack, preserves_flags)
        );
    }

    if syscall::failed(ret) {
        Err(::syscall::debug::WriteError::from_syscall_code(
            ret as isize,
        ))
    } else {
        Ok(ret)
    }
}
