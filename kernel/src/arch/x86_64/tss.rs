use macros::{init, per_cpu};

use crate::{arch::x86_64, library::lock::spin::Spinlock};

/// Represents the Task State Segment (TSS) structure. It is used by the
/// interrupt to determine which stack to use when handling an interrupt and
/// to determine which I/O ports are available to the running process.
/// See Intel Vol. 3A §7.2 for more details.
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    stacks: [u64; 3],
    reserved_2: u64,
    ist: [u64; 7],
    reserved_3: u64,
    reserved_4: u16,
    iomap_base: u16,
}

impl Default for TaskStateSegment {
    #[allow(clippy::cast_possible_truncation)]
    fn default() -> Self {
        Self {
            reserved_1: 0,
            stacks: [0; 3],
            reserved_2: 0,
            ist: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: core::mem::size_of::<Self>() as u16,
        }
    }
}

/// The TSS used by the kernel. Each CPU core has its own TSS, so this structure is
/// not shared between CPU cores.
#[per_cpu]
static TSS: Spinlock<TaskStateSegment> = Spinlock::new(TaskStateSegment::default());

/// Initialize the TSS for the current CPU core.
///
/// # Safety
/// The caller must call this function only once during the initialization of
/// the kernel and after the per-CPU data has been initialized as well as the
/// GDT structure.
#[init]
pub unsafe fn setup() {
    x86_64::gdt::load_tss(core::ptr::from_ref(&*TSS.local().lock()));
    x86_64::instr::ltr(x86_64::gdt::Selector::TSS.value());
}

/// Set the kernel stack for the current CPU core. This will be used by the
/// interrupt handler when a trap occurs in user mode and the CPU needs to
/// switch to the kernel stack before pushing the interrupt frame on the stack.
///
/// # Safety
/// The caller must ensure that the provided stack is valid and will remain
/// valid until another stack is set for the current CPU core. The stack
/// should be properly aligned, big enough and accessible from both read and
/// write operations.
pub unsafe fn set_kernel_stack(rsp: u64) {
    TSS.local().lock().stacks[0] = rsp;
}
