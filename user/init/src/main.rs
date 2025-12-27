#![no_std]
#![no_main]

#[xstd::main]
pub fn main() {
    let _echo = connect_until_success("echo");
    loop {
        xstd::task::yield_now();
    }
}

/// Connects to a service by its name, retrying until successful. This is
/// useful for services that may not be immediately available, such as during
/// system startup.
pub fn connect_until_success(name: &str) -> usize {
    loop {
        match xstd::service::connect(name) {
            Ok(handle) => return handle,
            Err(_) => xstd::task::yield_now(),
        }
    }
}
