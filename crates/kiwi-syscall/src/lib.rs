//! Kiwi Syscall Library. This crate provides definitions and utilities for
//! interacting with the Kiwi operating system's syscall interface. It is
//! used by both user-space applications and the kernel to reduce code
//! duplication that could lead to inconsistencies between definitions
//! used in user space and those in the kernel that could cause subtle bugs
//! if they get out of sync.
#![no_std]

pub mod ipc;
pub mod service;

/// Enumeration of supported syscall operations by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SyscallOp {
    /// No operation syscall, used for testing purposes.
    Nop = 0,

    /// Exit the current task.
    TaskExit = 1,

    /// Yield the current task's execution.
    TaskYield = 2,

    /// Register a new service.
    ServiceRegister = 3,

    /// Unregister a service.
    ServiceUnregister = 4,

    /// Connect to a service.
    ServiceConnect = 5,

    /// Send an IPC message
    IpcSend = 6,

    /// Receive an IPC message
    IpcReceive = 7,

    /// Reply to an IPC message
    IpcReply = 8,

    /// Used for representing an unknown or unsupported syscall operation. It
    /// cannoy be used in actual syscalls.
    Unknown = u32::MAX,
}

impl From<usize> for SyscallOp {
    fn from(value: usize) -> Self {
        match u32::try_from(value).unwrap_or(u32::MAX) {
            0 => SyscallOp::Nop,
            1 => SyscallOp::TaskExit,
            2 => SyscallOp::TaskYield,
            3 => SyscallOp::ServiceRegister,
            4 => SyscallOp::ServiceUnregister,
            5 => SyscallOp::ServiceConnect,
            6 => SyscallOp::IpcSend,
            7 => SyscallOp::IpcReceive,
            8 => SyscallOp::IpcReply,
            _ => SyscallOp::Unknown,
        }
    }
}
