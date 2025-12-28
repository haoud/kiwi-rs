/// The syscall numbers used by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallNumber {
    Nop = 0,
    TaskExit = 1,
    TaskYield = 2,
    ServiceRegister = 3,
    ServiceUnregister = 4,
    ServiceConnect = 5,
    IpcSend = 6,
    IpcReceive = 7,
    IpcReply = 8,
}

/// A trait that help to convert syscall return codes into specific error
/// types for better error handling.
pub trait SyscallCode {
    /// Converts a syscall error code into a specific error type. If the
    /// code does not match any known error, it should default to a generic
    /// unknown error. This can happen if the syscall interface is extended
    /// in the future and this library is not updated accordingly, or if
    /// this function is used when the syscall did not actually fail.
    fn from_syscall_code(code: isize) -> Self;
}

/// Checks if the given syscall return code indicates a failure. Code between
/// -1 and -255 (inclusive) are considered error codes.
pub fn failed(code: usize) -> bool {
    (code as isize) < 0 && (code as isize) >= -255
}
