use crate::{
    arch::trap::Resume,
    future, ipc,
    user::{self, syscall::SyscallReturnValue},
};

impl From<ipc::service::ServiceRegisterError> for ::syscall::service::RegisterError {
    fn from(value: ipc::service::ServiceRegisterError) -> Self {
        match value {
            ipc::service::ServiceRegisterError::NameNotAvailable => {
                ::syscall::service::RegisterError::NameNotAvailable
            }
            ipc::service::ServiceRegisterError::TaskAlreadyRegistered => {
                ::syscall::service::RegisterError::TaskAlreadyRegistered
            }
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
) -> Result<SyscallReturnValue, ::syscall::service::RegisterError> {
    let name = user::string::String::new(name_ptr, name_len)
        .ok_or(::syscall::service::RegisterError::BadName)?
        .fetch()
        .map_err(|_| ::syscall::service::RegisterError::BadName)?;
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
pub fn unregister() -> Result<SyscallReturnValue, ::syscall::service::UnregisterError> {
    log::warn!("Service unregistration is not yet implemented");
    Err(::syscall::service::UnregisterError::NotImplemented)
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
) -> Result<SyscallReturnValue, ::syscall::service::ConnectionError> {
    let name = user::string::String::new(name_ptr, name_len)
        .ok_or(::syscall::service::ConnectionError::BadName)?
        .fetch()
        .map_err(|_| ::syscall::service::ConnectionError::BadName)?;
    let service_id =
        ipc::service::lookup(&name).ok_or(::syscall::service::ConnectionError::ServiceNotFound)?;

    Ok(SyscallReturnValue {
        resume: Resume::Continue,
        value: usize::from(service_id),
    })
}
