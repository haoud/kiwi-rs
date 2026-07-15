use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use alloc::boxed::Box;

use crate::{
    arch::x86_64::{
        cpu::{self, rflags},
        gdt,
    },
    scheduler::{self, TaskState},
};

core::arch::global_asm!(include_str!("asm/thread.asm"));

unsafe extern "C" {
    fn thread_resume(context: *mut cpu::InterruptFrame) -> *mut cpu::InterruptFrame;
    fn thread_exit() -> !;
}

/// A structure that represents the context of a thread.
#[derive(Default, Debug)]
pub struct LocalContext {
    /// An option containing a pointer to the saved trap frame of the thread.
    /// This is used to store the state of the thread when it is interrupted,
    /// so that it can be resumed later. If the thread is currently running,
    /// this field is set to `None`, since the state of the thread is stored
    /// in the CPU registers and cannot be accessed directly.
    cpu: Option<NonNull<cpu::InterruptFrame>>,

    /// The kernel stack of the thread. This is used to store the saved trap
    /// frame when the thread is interrupted. It is never used directly, but
    /// must be kept alive for the lifetime of the thread, since the saved trap
    /// frame is stored on the stack and must not be deallocated while the
    /// thread is running.
    #[allow(dead_code)]
    kstack: KernelStack,
}

/// SAFETY: The `LocalContext` is Send because it only contains a pointer to
/// the saved trap frame into his own kernel stack. Since a context is only
/// used by a single thread at a time when it is not running, it is safe to
/// send it to another thread (for exemple, for load balancing).
unsafe impl Send for LocalContext {}

impl LocalContext {
    /// Creates a new `LocalContext` with the given function as the entry point
    /// and the given parameter as the argument to the function that will be
    /// stored in the `rdi` register.
    fn new(function: usize, param: usize) -> Self {
        let mut kstack = KernelStack::new();
        let cpu = cpu::InterruptFrame {
            rflags: rflags::Flags::IF | rflags::Flags::RESERVED,
            rip: cpu::Register::from(function),
            rdi: cpu::Register::from(param),
            rsp: cpu::Register::from(kstack.top().addr()),
            cs: cpu::Register::from(u16::from(gdt::Selector::KERNEL_CODE)),
            ss: cpu::Register::from(u16::from(gdt::Selector::KERNEL_DATA)),
            ..Default::default()
        };

        // SAFETY: The pointer is guaranteed to be non-null and properly
        // aligned, and the offset is within the bounds of the allocated memory.
        let cpu_ptr = unsafe {
            NonNull::new_unchecked(
                kstack
                    .top_mut()
                    .cast::<cpu::InterruptFrame>()
                    .wrapping_sub(1),
            )
        };

        // Copy the interrupt frame into the interrupt stack to simulate a trap
        // to the kernel, allowing the thread to be resumed for the first time.
        //
        // SAFETY: The pointer is guaranteed to be non-null and properly
        // aligned, and the offset is within the bounds of the allocated
        // memory. Furthermore, the memory is accessible and valid for writes
        // since the kernel stack is declared as mutable, and we are the sole
        // owner of the kernel stack at this point, so no other thread can
        // access it.
        unsafe {
            kstack
                .top_mut()
                .cast::<cpu::InterruptFrame>()
                .wrapping_sub(1)
                .write(cpu);
        };

        Self {
            cpu: Some(cpu_ptr),
            kstack,
        }
    }

    /// Creates a new `LocalContext` with the given function as the entry
    /// point of the kernel thread.
    pub fn from_function(function: fn() -> !) -> Self {
        Self::new(function as usize, 0)
    }

    /// Creates a new `LocalContext` with the given function as the entry
    /// point of the kernel thread, and the given parameter as the argument
    /// to the function.
    pub fn from_closure(function: KernelThreadEntry, parameter: Box<KernelThreadStart>) -> Self {
        Self::new(function as usize, Box::into_raw(parameter).addr())
    }
}

/// A kernel stack.
#[derive(Debug)]
pub struct KernelStack {
    inner: Box<MaybeUninit<KernelStackData>>,
}

impl KernelStack {
    /// The default size of the kernel stack in bytes. It must be a multiple
    /// of the page size (4096 bytes), or things could break horribly !
    pub const SIZE: usize = 4096 * 4;

    /// Creates a new kernel stack. The stack is allocated on the heap and is
    /// uninitialized, since Rust does not need the stack to be initialized
    /// with any specific values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Box::new_uninit(),
        }
    }

    /// Returns a mutable pointer to the top of the kernel stack. The top of
    /// the stack is the highest address, when the stack starts at the bottom
    /// and grows downwards.
    #[must_use]
    pub fn top_mut(&mut self) -> *mut KernelStackData {
        // SAFETY: All the requirements to the `add` method are met, since the
        // pointer is non-null and properly aligned, and the offset is within
        // the bounds of the allocated memory.
        unsafe { self.inner.as_mut_ptr().cast::<KernelStackData>().add(1) }
    }

    /// Returns a const pointer to the top of the kernel stack. The top of the
    /// stack is the highest address, when the stack starts at the bottom and
    /// grows downwards.
    #[must_use]
    pub fn top(&self) -> *const KernelStackData {
        // SAFETY: All the requirements to the `add` method are met, since the
        // pointer is non-null and properly aligned, and the offset is within
        // the bounds of the allocated memory.
        unsafe { self.inner.as_ptr().cast::<KernelStackData>().add(1) }
    }

    /// Returns a pointer to the bottom of the kernel stack.
    #[must_use]
    pub fn bottom(&self) -> *const u8 {
        self.inner.as_ptr().cast::<u8>()
    }
}

impl Default for KernelStack {
    fn default() -> Self {
        Self::new()
    }
}

/// A structure that represents the data stored on the kernel stack. We need
/// to create a separate structure for the kernel stack data to ensure that
/// the stack is properly aligned on a 4096-byte boundary.
/// Therefore, this structure dereferences into its inner data since the
/// structure itself is just required to the alignment constraint.
#[derive(Debug)]
#[repr(align(4096))]
pub struct KernelStackData {
    data: [u8; KernelStack::SIZE],
}

impl Deref for KernelStackData {
    type Target = [u8; KernelStack::SIZE];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for KernelStackData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct Thread {
    context: LocalContext,
}

impl Thread {
    /// Creates a new thread with the given function as the entry point. The
    /// thread is created with a new kernel stack and an uninitialized context.
    ///
    /// # Safety
    /// The function must not return, as it is expected to run indefinitely.
    pub fn kernel(function: fn() -> !) -> Self {
        Self {
            context: LocalContext::from_function(function),
        }
    }

    /// Creates a new kernel thread with the given entry point that takes a
    /// pointer to a `KernelThreadStart` struct as an argument.
    /// The `KernelThreadStart` struct contains a closure that should be
    /// executed by the kernel thread.
    #[must_use]
    pub fn from_closure(function: KernelThreadEntry, closure: Box<KernelThreadClosure>) -> Self {
        Self {
            context: LocalContext::from_closure(function, Box::new(KernelThreadStart { closure })),
        }
    }

    /// Returns a mutable reference to the thread's context.
    #[must_use]
    pub fn context_mut(&mut self) -> &mut LocalContext {
        &mut self.context
    }

    /// Returns a reference to the thread's context.
    #[must_use]
    pub fn context(&self) -> &LocalContext {
        &self.context
    }
}

/// The structure that is passed to the kernel thread entry point in order to
/// have an API similar to the one of `std::thread::spawn`.
pub struct KernelThreadStart {
    /// The closure that will be executed by the kernel thread.
    closure: Box<KernelThreadClosure>,
}

impl KernelThreadStart {
    /// Run the closure that was passed to the kernel thread entry point.
    pub fn run(self) {
        (self.closure)();
    }
}

/// A type alias for the entry point of a kernel thread. The function takes a
/// pointer to a `KernelThreadStart` struct as an argument, which contains the
/// closure to be executed by the kernel thread.
pub type KernelThreadEntry = unsafe extern "C" fn(*mut KernelThreadStart) -> !;

/// A type alias for the closure to be executed by a kernel thread. The closure
/// must be `Send` and have a `'static` lifetime, as it will be executed
/// in a separate thread context and may outlive the current thread.
pub type KernelThreadClosure = dyn FnOnce() + Send + 'static;

/// Executes the given thread until it is interrupted or exits. When this
/// happens, the thread's state is saved and this function returns, allowing
///  the scheduler to switch to another thread.
///
/// # Panics
/// This function panics if the thread does not have a saved trap frame in its
/// context, which indicates that the thread is not in a state that can be
/// executed (for example, if it has exited or if it is already running).
pub fn execute(thread: &mut Thread) {
    let saved_trap_frame = thread
        .context
        .cpu
        .take()
        .expect("Trying to execute a thread that has no saved trap frame");

    // Change the interrupt stack pointer to the thread's kernel stack and
    // resume the thread by restoring its saved trap frame. The thread will
    // run until a trap occurs and will return to this function by using
    // assembly black magic.
    // SAFETY: The saved trap frame is guaranteed to be valid, non-null and
    // properly aligned, and remains valid until the thread is interrupted or
    // exits.
    let trap_frame_ptr = unsafe { thread_resume(saved_trap_frame.as_ptr()) };

    // Put the saved trap frame back into the thread's context, so that
    // it can be resumed later. If the pointer returned by thread_resume
    // is null, it means that the thread has exited/failed and should not
    // be resumed again.
    thread.context.cpu = NonNull::new(trap_frame_ptr);
}

/// Exits the current thread and switches to another thread.
///
/// # Panics
/// This function panics if it is called during the initialization of the
/// kernel, or if it is called from kernel code that is not running in a
/// thread context.
pub fn exit() -> ! {
    scheduler::change_current_task_state(TaskState::Exited);
    scheduler::set_need_reschedule(true);

    // SAFETY: This function never returns, as it will switch to another thread
    // and should not cause any undefined behavior or memory safety issues.
    unsafe { thread_exit() }
}
