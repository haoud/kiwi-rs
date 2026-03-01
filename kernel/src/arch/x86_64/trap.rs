use core::sync::atomic::{AtomicBool, Ordering};

use macros::init;

use crate::{
    arch::x86_64::{self, gdt},
    library::lock::spin::Spinlock,
};

core::arch::global_asm!(include_str!("asm/trap.asm"));

unsafe extern "C" {
    static interrupt_handlers: [TrapTrampoline; 256];
}

/// An IDT descriptor. An IDT descriptor is a 16 bytes structure that contains
/// the address of the handler, the segment selector and the descriptor flags.
/// For more details, see the Intel manual (Volume 3, Chapter 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Descriptor {
    offset_low: u16,
    selector: u16,
    flags: u16,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl Descriptor {
    /// Create a new uninitialized IDT descriptor. If this descriptor is read
    /// by the CPU, it will result in a general protection fault, and a triple
    /// fault and a reboot of the computer if the CPU doesn't have a proper
    /// exception handler set up.
    #[must_use]
    pub const fn uninitialized() -> Self {
        Self {
            selector: 0,
            offset_high: 0,
            offset_mid: 0,
            offset_low: 0,
            flags: 0,
            zero: 0,
        }
    }

    /// Update the IDT descriptor with the provided handler address. It will set
    /// the segment selector to the kernel code segment and the flags to 0x8E00
    /// (present, interrupt gate, DPL=0).
    #[allow(clippy::cast_possible_truncation)]
    pub fn write(&mut self, trampoline: &'static TrapTrampoline) {
        let address = trampoline.as_ptr().addr() as u64;
        self.selector = gdt::Selector::KERNEL_CODE.value();
        self.offset_high = (address >> 32) as u32;
        self.offset_mid = (address >> 16) as u16;
        self.offset_low = address as u16;
        self.flags = 0x8E00;
        self.zero = 0;
    }
}

/// An IDT register. It is used to load the IDT in the current CPU core using
/// the `lidt` instruction. It is a simple wrapper around a 16-bits limit and
/// a 64-bits base address that represents the IDT in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Register {
    limit: u16,
    base: u64,
}

impl Register {
    /// Creates a new IDT register with the global IDT table.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn new() -> Self {
        Self {
            limit: (core::mem::size_of::<[Descriptor; 256]>() - 1) as u16,
            base: TABLE.lock().as_ptr() as u64,
        }
    }

    /// Load the IDT register with the current table in the current CPU core
    /// using the `lidt` instruction.
    ///
    /// # Safety
    /// The caller must ensure that the IDT was properly initialized before
    /// calling this function, and that the IDT will remain valid for the
    /// entire lifetime of the kernel or until another IDT is loaded in the
    /// CPU.
    pub unsafe fn load(&self) {
        x86_64::instr::lidt(core::ptr::from_ref::<Self>(self) as usize);
    }
}

impl Default for Register {
    fn default() -> Self {
        Self::new()
    }
}

/// A trap trampoline. It is a simple wrapper around the 16 bytes of the
/// handler code defined in the `asm/trap.asm` file. It is used to get a
/// pointer to the start of the handler code, which is needed to set up the
/// IDT descriptors.
#[repr(C, align(16))]
pub struct TrapTrampoline {
    opcodes: [u8; 16],
}

impl TrapTrampoline {
    /// Returns a pointer to itself.
    #[must_use]
    pub const fn as_ptr(&'static self) -> *const Self {
        core::ptr::from_ref::<Self>(self)
    }
}

/// The global IDT table, shared between all CPU cores.
static TABLE: Spinlock<[Descriptor; 256]> = Spinlock::new([Descriptor::uninitialized(); 256]);

/// A flag to ensure that the IDT is only initialized once. It is set to true
/// after the first call to `setup`, and it is used to only initialize the IDT
/// once. We could still allow the IDT to be initialized multiple times, but it
/// is not necessary and it is better to prevent it.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the IDT and loads it in the current CPU core. If the IDT is
/// not yet initialized, it will initialize it with the handlers defined in the
/// `asm/trap.asm` file.
///
/// # Safety
/// The caller must call this function only once during the initialization of
/// the kernel and after the per-CPU data has been initialized.
#[init]
pub unsafe fn setup() {
    if !INITIALIZED.swap(true, Ordering::SeqCst) {
        TABLE
            .lock()
            .iter_mut()
            .zip(&interrupt_handlers)
            .for_each(|(descriptor, handler)| {
                descriptor.write(handler);
            });
    }

    Register::default().load();
}

/// The trap handler. It is called by the assembly code defined in the
/// `asm/trap.asm` file when a trap occurs. It is common for all traps,
/// allowing to handle all traps in a single place.
///
/// # Safety
/// This function should only be called by the assembly code defined in the
/// `asm/trap.asm` file, and you should not use it directly in Rust code. It
/// is marked as public because it must be accessible from the assembly code,
/// but it is not intended to be used by other Rust code. This is why it is
/// marked as `unsafe`: calling this function directly from Rust code is UB.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn trap_handler() {}
