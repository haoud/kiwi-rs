#![no_std]
#![no_main]

/// An initialization service that connects to the "echo" service, sends a
/// message, and verifies the response. If the response matches the sent
/// message, it exits with a success code; otherwise, it exits with an error
/// code. This service demonstrates basic IPC communication and service
/// interaction.
#[xstd::main]
pub fn main() {
    let echo = connect_until_success("echo");
    let reply = xstd::ipc::send(echo, 42, b"Hello, world!").unwrap();
    let payload = &reply.payload[..reply.payload_len];

    if reply.status == 42 && payload == b"Hello, world!" {
        _ = xstd::debug::write("Echo service responded correctly !");
        xstd::task::exit(0)
    } else {
        _ = xstd::debug::write("Echo service responded incorrectly !");
        _ = xstd::debug::write("Note: This is probably the kernel's fault :) ");
        xstd::task::exit(-1)
    }
}

/// Connects to a service by its name, retrying until successful. This is
/// useful for services that may not be immediately available, such as during
/// system startup.
pub fn connect_until_success(name: &str) -> usize {
    loop {
        match xstd::service::connect(name) {
            Ok(handle) => {
                _ = xstd::debug::write("Successfully connected to the service !");
                return handle;
            }
            Err(_) => {
                _ = xstd::debug::write("Failed to connect to the service, retrying...");
                xstd::task::yield_now()
            }
        }
    }
}
