use crate::syscall::{self, SyscallCode};

/// Errors that may occur during service registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceRegisterError {
    /// An unknown error occurred.
    Unknown = 0,

    /// An invalid name was provided. It could be due to an invalid pointer,
    /// length, or the name not being valid UTF-8.
    BadName = 1,

    /// The service name is already taken by another service.
    NameNotAvailable = 2,

    /// The task is already registered as a service provider and cannot
    /// be registered again.
    TaskAlreadyRegistered = 3,
}

impl SyscallCode for ServiceRegisterError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => ServiceRegisterError::BadName,
            2 => ServiceRegisterError::NameNotAvailable,
            3 => ServiceRegisterError::TaskAlreadyRegistered,
            _ => ServiceRegisterError::Unknown,
        }
    }
}

/// Errors that may occur during service unregistration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceUnregisterError {
    /// An unknown error occurred.
    Unknown = 0,

    /// The service unregistration feature is not yet implemented.
    NotImplemented = 1,
}

impl SyscallCode for ServiceUnregisterError {
    fn from_syscall_code(code: isize) -> Self {
        match -code {
            1 => ServiceUnregisterError::NotImplemented,
            _ => ServiceUnregisterError::Unknown,
        }
    }
}

/// Registers the current task as a service provider with the given name. The
/// name must be a valid UTF-8 string and unique among all registered services.
///
/// # Errors
/// This function returns a [`ServiceRegisterError`] if the registration fails
/// for any reason, such as an invalid name or if the name is already taken by
/// another service.
pub fn register(name: &str) -> Result<(), ServiceRegisterError> {
    let ret;
    unsafe {
        core::arch::asm!("ecall",
            in("a7") 3,                 // syscall number for service_register
            in("a0") name.as_ptr(),     // pointer to the service name
            in("a1") name.len(),        // length of the service name
            lateout("a0") ret,          // return value
            options(nostack, preserves_flags)
        );
    }

    if syscall::failed(ret) {
        Err(ServiceRegisterError::from_syscall_code(ret as isize))
    } else {
        Ok(())
    }
}

/// Unregisters the current task's service.
///
/// # Errors
/// This function returns a [`ServiceUnregisterError`] if the unregistration
/// fails for any reason.
pub fn unregister() -> Result<(), ServiceUnregisterError> {
    let ret;
    unsafe {
        core::arch::asm!("ecall",
            in("a7") 4,         // syscall number for service_unregister
            lateout("a0") ret,  // return value
            options(nostack, preserves_flags)
        );
    }

    if syscall::failed(ret) {
        Err(ServiceUnregisterError::from_syscall_code(ret as isize))
    } else {
        Ok(())
    }
}
