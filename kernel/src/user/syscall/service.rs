use crate::{
    arch::trap::Resume,
    future, ipc,
    user::{self, syscall::SyscallReturnValue},
};

/// Errors that may occur during service registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceRegisterError {
    /// An invalid name was provided. It could be due to an invalid pointer,
    /// length, or the name not being valid UTF-8.
    BadName = 1,

    /// The service name is already taken by another service.
    NameNotAvailable = 2,

    /// The task is already registered as a service provider and cannot
    /// be registered again.
    TaskAlreadyRegistered = 3,
}

impl From<ServiceRegisterError> for isize {
    fn from(error: ServiceRegisterError) -> Self {
        match error {
            ServiceRegisterError::BadName => 1,
            ServiceRegisterError::NameNotAvailable => 2,
            ServiceRegisterError::TaskAlreadyRegistered => 3,
        }
    }
}

impl From<ipc::service::ServiceRegisterError> for ServiceRegisterError {
    fn from(value: ipc::service::ServiceRegisterError) -> Self {
        match value {
            ipc::service::ServiceRegisterError::NameNotAvailable => {
                ServiceRegisterError::NameNotAvailable
            }
            ipc::service::ServiceRegisterError::TaskAlreadyRegistered => {
                ServiceRegisterError::TaskAlreadyRegistered
            }
        }
    }
}

/// Errors that may occur during service unregistration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceUnregisterError {
    /// The service unregistration feature is not yet implemented.
    NotImplemented = 1,
}

impl From<ServiceUnregisterError> for isize {
    fn from(error: ServiceUnregisterError) -> Self {
        match error {
            ServiceUnregisterError::NotImplemented => 1,
        }
    }
}

/// Errors that may occur during service connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceConnectError {
    /// An invalid name was provided. It could be due to an invalid pointer,
    /// length, or the name not being valid UTF-8.
    BadName = 1,

    /// No service with the specified name exists.
    ServiceNotFound = 2,
}

impl From<ServiceConnectError> for isize {
    fn from(error: ServiceConnectError) -> Self {
        match error {
            ServiceConnectError::BadName => 1,
            ServiceConnectError::ServiceNotFound => 2,
        }
    }
}

/// Registers a new service with the given name pointer and length.
///
/// # Errors
/// This function returns `Ok(Resume::Continue)` if the service was registered
/// successfully. If there was an error during registration, it returns
/// an appropriate [`ServiceRegisterError`] describing the failure.
///
/// # Panics
/// This function may panic if it encounters an unrecoverable error while
/// handling the syscall. This includes, but is not limited to:
/// - The executor does not have a current task when required (this should
///   never happen since service registration must be done within a task
///   context).
pub fn register(
    name_ptr: *mut u8,
    name_len: usize,
) -> Result<SyscallReturnValue, ServiceRegisterError> {
    let name = user::string::String::new(name_ptr, name_len)
        .ok_or(ServiceRegisterError::BadName)?
        .fetch()
        .map_err(|_| ServiceRegisterError::BadName)?;
    let id = future::executor::current_task_id().unwrap();

    ipc::service::register(name, id)?;
    Ok(SyscallReturnValue {
        resume: Resume::Continue,
        value: 0,
    })
}

/// Unregisters the current service.
///
/// # Errors
/// This function always returns an error indicating that the service
/// unregistration feature is not yet implemented. This is because the proper
/// approach to handle service unregistration is unclear at this time. For
/// example, we need to consider what happens to existing connections to
/// the service, and how to handle pending requests. Therefore, this function
/// is a placeholder for future implementation.
pub fn unregister() -> Result<SyscallReturnValue, ServiceUnregisterError> {
    log::warn!("Service unregistration is not yet implemented");
    Err(ServiceUnregisterError::NotImplemented)
}

/// Connects to a service by its name.
///
/// # Errors
/// This function returns `Ok(Resume::ReturnValue(service_id))` if the service
/// was found and connected successfully. If there was an error during connection,
/// it returns an appropriate [`ServiceConnectError`] describing the failure.
///
/// The `service_id` can be used for subsequent IPC operations with the
/// connected service. Since this is not really a connection in the traditional
/// sense, but rather a lookup of the service ID, no actual connection state
/// is maintained, and thus no disconnection is necessary.
pub fn connect(
    name_ptr: *mut u8,
    name_len: usize,
) -> Result<SyscallReturnValue, ServiceConnectError> {
    let name = user::string::String::new(name_ptr, name_len)
        .ok_or(ServiceConnectError::BadName)?
        .fetch()
        .map_err(|_| ServiceConnectError::BadName)?;

    let service_id = ipc::service::lookup(&name).ok_or(ServiceConnectError::ServiceNotFound)?;

    Ok(SyscallReturnValue {
        resume: Resume::Continue,
        value: usize::from(service_id),
    })
}
