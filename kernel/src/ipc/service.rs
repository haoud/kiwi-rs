use crate::future;
use alloc::string::String;
use hashbrown::HashMap;

/// A global registry for services provided by tasks. It maps task identifiers
/// to their corresponding service names.
static SERVICE_REGISTRY: spin::Once<spin::Mutex<HashMap<String, future::task::Identifier>>> =
    spin::Once::new();

/// Errors that may occur during service registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceRegisterError {
    /// The service name is already taken.
    NameNotAvailable,

    /// The task is already registered as a service provider.
    TaskAlreadyRegistered,
}

/// Initializes the service registry.
pub fn setup() {
    SERVICE_REGISTRY.call_once(|| spin::Mutex::new(HashMap::new()));
}

/// Registers a new service with the given name and task identifier.
///
/// # Errors
/// This function may fail and return:
/// - [`ServiceRegisterError::NameNotAvailable`] if the service name is
///   already taken.
/// - [`ServiceRegisterError::TaskAlreadyRegistered`] if the task is already
///   registered.
///
/// # Panics
/// This function may panic if the service registry has not been initialized
/// by calling `setup()` beforehand. This should never happen, and indicates a
/// bug in the kernel.
pub fn register(name: String, id: future::task::Identifier) -> Result<(), ServiceRegisterError> {
    let mut registry = SERVICE_REGISTRY.get().unwrap().lock();

    // Verify that the task is not already registered. It iterates through
    // the existing services in the registry and checks if any of them match
    // the provided name. This is kinda inefficient, but service registration
    // is not expected to be a frequent operation so this should be fine
    if registry.values().any(|&task_id| task_id == id) {
        return Err(ServiceRegisterError::TaskAlreadyRegistered);
    }

    // Verify that the service name is not already taken.
    if registry.contains_key(&name) {
        return Err(ServiceRegisterError::NameNotAvailable);
    }

    registry.insert(name, id);
    Ok(())
}

/// Looks up a service by its name and returns the corresponding task. If no
/// such service exists, `None` is returned.
///
/// # Panics
/// This function may panic if the service registry has not been initialized
/// by calling `setup()` beforehand. This should never happen, and indicates a
/// bug in the kernel.
pub fn lookup(name: &str) -> Option<future::task::Identifier> {
    SERVICE_REGISTRY.get().unwrap().lock().get(name).copied()
}
