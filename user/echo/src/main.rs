#![no_std]
#![no_main]

/// A simple echo service that replies to any received message with the same
/// payload and status code. It registers itself under the name "echo" and enters
/// a loop to handle all incoming messages.
///
/// This service can be used for testing IPC mechanisms by sending messages
/// to it and verifying that the replies match the sent messages.
#[xstd::main]
pub fn main() {
    xstd::service::register("echo").unwrap();
    loop {
        let msg = xstd::ipc::receive().unwrap();
        _ = xstd::debug::write("Echo service received a message, replying...");
        _ = xstd::ipc::reply(msg.sender, msg.kind, &msg.payload[..msg.payload_len]);
    }
}
