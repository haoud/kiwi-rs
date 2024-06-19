#![allow(dead_code)]

extern crate alloc;

#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "riscv64")]
use riscv64 as target;

pub mod generic;
pub use generic::*;

extern "Rust" {
    /// The architecture-independent entry point for the kernel. This function
    /// should be called by the architecture-specific entry point after the
    /// architecture-specific initialization is complete.
    fn kiwi(memory: generic::memory::UsableMemory) -> !;
}
