/// Errors that can occur when writing to the kernel debug output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// An unknown error occurred.
    Unknown = 0,

    /// An invalid name was provided. It could be due to an invalid pointer,
    /// length, or the name not being valid UTF-8.
    BadName = 1,

    /// No output device is available to write the debug output.
    NoOutputAvailable = 2,
}

impl From<WriteError> for isize {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::Unknown => 0,
            WriteError::BadName => 1,
            WriteError::NoOutputAvailable => 2,
        }
    }
}
