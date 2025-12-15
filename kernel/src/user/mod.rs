use crate::arch::target::addr::{Virtual, virt::User};

pub mod elf;
pub mod syscall;

/// The top address of the user stack, exclusive. This is located just below the
/// last page of the user address space. We don't use the very last page since it
/// already has caused security issues in the past in the Linux kernel.
pub const USER_STACK_TOP: Virtual<User> = Virtual::<User>::new(0x0000_003F_FFFF_F000);

/// By default, each task has a 64 kiB stack.
pub const USER_STACK_SIZE: usize = 0x10000;

/// The bottom address of the user stack, inclusive.
pub const USER_STACK_BOTTOM: Virtual<User> =
    Virtual::<User>::new(USER_STACK_TOP.as_usize() - USER_STACK_SIZE);
