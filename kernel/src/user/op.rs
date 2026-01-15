use core::sync::atomic::{AtomicBool, Ordering};
use zerocopy::{FromBytes, IntoBytes};

use crate::arch::{self, thread::Thread};

/// The `USER_OPERATION` variable is used to signal if the current CPU is
/// performing a user operation or not. This is useful to not panic when a
/// unrecoverable page fault occurs in kernel space: if an user operation
/// was in progress, then we can try to kill the process because it is likely
/// that the fault was caused by the user process who tried to access invalid
/// memory or gave an invalid pointer to the kernel. If no user operation was
/// in progress, then we can't do anything and we must panic.
static USER_OPERATION: AtomicBool = AtomicBool::new(false);

/// Checks if the current CPU is currently performing a user operation.
#[must_use]
pub fn in_operation() -> bool {
    USER_OPERATION.load(Ordering::Relaxed)
}

/// Copy `len` bytes from the given source address to the given destination
/// address. This function should only be used to copy data from user space
/// to kernel space. If you want to copy data from kernel space to user space,
/// then you should use [`copy_to`].
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that
/// could possibly be invalid: it is the caller's responsibility to ensure
/// that the pointer is valid and does not overlap with kernel space. However,
/// the caller does not need to ensure that the user memory is readable, as
/// this function will handle page faults and kill the process if necessary.
pub unsafe fn copy_from<T: FromBytes>(thread: &Thread, src: *const T, dst: *mut T, len: usize) {
    thread.root_table().set_current();
    perform_user_operation(|| {
        core::ptr::copy_nonoverlapping(src, dst, len);
    });
}

/// Copy `len` bytes from the given source address to the given destination
/// address. This function should only be used to copy data from kernel space
/// to user space. If you want to copy data from user space to kernel space,
/// then you should use [`copy_from`].
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that
/// could possibly be invalid: it is the caller's responsibility to ensure
/// that the pointer is valid and does not overlap with kernel space. However,
/// the caller does not need to ensure that the user memory is writable, as
/// this function will handle page faults and kill the process if necessary.
pub unsafe fn copy_to<T: IntoBytes>(thread: &Thread, src: *const T, dst: *mut T, len: usize) {
    thread.root_table().set_current();
    perform_user_operation(|| {
        core::ptr::copy_nonoverlapping(src, dst, len);
    });
}

/// Write the given value to the given address. This function is implemented by
/// a simple call to [`copy_from`] with the same source and destination
/// address and a length of 1. This will copy one `T` from the userland memory
/// to the kernel.
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that
/// could possibly be invalid: it is the caller's responsibility to ensure
/// that the pointer is valid and does not overlap with kernel space. However,
/// the caller does not need to ensure that the memory is readable, as this
/// function will handle page faults and kill the process if necessary.
pub unsafe fn read<T: FromBytes>(thread: &Thread, src: *const T, dst: *mut T) {
    copy_from(thread, src, dst, 1);
}

/// Write the given value to the given address. This function is implemented by
/// a simple call to [`copy_to`] with the same source and destination address
/// and a length of 1. This will copy one `T` from the kernel to the userland
/// memory.
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that
/// could possibly be invalid: it is the caller's responsibility to ensure that
/// the pointer is valid and does not overlap with kernel space. However, the
/// caller does not need to ensure that the memory is writable, as this function
/// will handle page faults and kill the task if the access is invalid.
pub unsafe fn write<T: IntoBytes>(thread: &Thread, src: *const T, dst: *mut T) {
    copy_to(thread, src, dst, 1);
}

/// Signal that the current CPU has started an user operation. This will enable
/// access to user pages without causing a page fault, and will set the internal
/// flag to indicate that an user operation is in progress (see [`in_operation`]).
///
/// # Panics
/// This function will panic if an user operation was already in progress.
fn start_user_operation() {
    let was_in_operation = USER_OPERATION.swap(true, Ordering::Relaxed);
    arch::mmu::allow_user_page_access();
    assert!(
        !was_in_operation,
        "Nested user operations are not supported"
    );
}

/// Signal that the current CPU has finished an user operation. This will disable
/// access to user pages, and will clear the internal flag to indicate that no
/// user operation is in progress (see [`in_operation`]).
///
/// # Panics
/// This function will panic if no user operation was in progress.
fn end_user_operation() {
    arch::mmu::forbid_user_page_access();
    let was_in_operation = USER_OPERATION.swap(false, Ordering::Relaxed);
    assert!(
        was_in_operation,
        "No user operation was in progress, cannot end it"
    );
}

/// Executes the given function while signaling that the current CPU is
/// performing a user operation. During the execution of the closure,
/// preemption and interrupts are disabled to avoid race conditions.
///
/// # Panics
/// This function will panic if this function is used recursively (calling
/// this function when the CPU core is already performing an user operation).
fn perform_user_operation<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    arch::generic::irq::without(|| {
        start_user_operation();
        let ret = f();
        end_user_operation();
        ret
    })
}
