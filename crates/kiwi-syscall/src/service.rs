/// Errors that may occur during service registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError {
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

impl From<RegisterError> for isize {
    fn from(error: RegisterError) -> Self {
        match error {
            RegisterError::Unknown => 0,
            RegisterError::BadName => 1,
            RegisterError::NameNotAvailable => 2,
            RegisterError::TaskAlreadyRegistered => 3,
        }
    }
}

/// Errors that may occur during service unregistration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnregisterError {
    /// An unknown error occurred.
    Unknown = 0,

    /// The service unregistration feature is not yet implemented.
    NotImplemented = 1,
}

impl From<UnregisterError> for isize {
    fn from(error: UnregisterError) -> Self {
        match error {
            UnregisterError::Unknown => 0,
            UnregisterError::NotImplemented => 1,
        }
    }
}

/// Errors that may occur during service connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionError {
    /// An unknown error occurred.
    Unknown = 0,

    /// An invalid name was provided. It could be due to an invalid pointer,
    /// length, or the name not being valid UTF-8.
    BadName = 1,

    /// No service with the specified name exists.
    ServiceNotFound = 2,
}

impl From<ConnectionError> for isize {
    fn from(error: ConnectionError) -> Self {
        match error {
            ConnectionError::Unknown => 0,
            ConnectionError::BadName => 1,
            ConnectionError::ServiceNotFound => 2,
        }
    }
}
