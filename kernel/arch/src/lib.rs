//! The architecture-specific code for the kernel. This crate is responsible
//! for abstracting the architecture-specific code from the rest of the kernel.
//! This allows the kernel to be compiled for different architectures without
//! changing the rest of the kernel code.
//!
//! # Adding a new architecture
//! To add a new architecture, simply copy the `generic` module and rename it
//! to the target architecture. Then, implement all the functions in the
//! module, following the requirements of the function in the comments.
//!
//! Finally, add the new module to the `lib.rs` file, and add a conditional
//! compilation block to include the new module only when the target
//! architecture is the new architecture.
//!
//! ```rust
//! #[cfg(target_arch = "new_arch")]
//! mod new_arch;
//! #[cfg(target_arch = "new_arch")]
//! use new_arch as target;
//! ```
//!
//! Congratulations! You have successfully added a new architecture
//! to the kernel.
#![no_std]
#![allow(dead_code)]
#![feature(panic_info_message)]

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
