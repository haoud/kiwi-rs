use bitfield::{BitMut, BitRangeMut};
use macros::{init, per_cpu};

use crate::{
    arch::x86_64::{self, cpu::Privilege, tss::TaskStateSegment},
    library::lock::spin::Spinlock,
};

/// The selector of a GDT entry. It is used to load the GDT entry in the CPU
/// by referencing its index in the GDT and the privilege level that we want
/// to use for this entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Selector(u16);

impl Selector {
    pub const KERNEL_CODE: Self = Self::new(1, Privilege::Kernel);
    pub const KERNEL_DATA: Self = Self::new(2, Privilege::Kernel);
    pub const KERNEL_STACK: Self = Self::KERNEL_DATA;

    pub const USER_CODE: Self = Self::new(3, Privilege::User);
    pub const USER_DATA: Self = Self::new(4, Privilege::User);
    pub const USER_STACK: Self = Self::USER_DATA;

    pub const TSS: Self = Self::new(6, Privilege::Kernel);

    /// Creates a new selector with the provided index and privilege level.
    #[must_use]
    pub const fn new(index: u16, privilege: Privilege) -> Self {
        Self((index << 3) | (privilege as u16 & 0b11))
    }

    /// Returns the value of the selector.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// A GDT register. It is used to load the GDT in the current CPU core using
/// the `lgdt` instruction. It is a simple wrapper around a 16-bits limit and
/// a 64-bits base address that represents the GDT in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Register {
    limit: u16,
    base: u64,
}

impl Register {
    /// Creates a new GDT register with the provided table. It will set the
    /// limit to the size of the table minus one and the base to the address
    /// of the table. The given table MUST stay in memory while it is loaded
    /// in the CPU !
    #[must_use]
    pub fn new(table: *const [Entry; 8]) -> Self {
        Self {
            #[allow(clippy::cast_possible_truncation)]
            limit: (core::mem::size_of::<[Entry; 8]>() - 1) as u16,
            base: table as u64,
        }
    }

    /// Loads the GDT register with the current table in the current CPU core
    /// using the `lgdt` instruction.
    ///
    /// # Safety
    /// The caller must ensure that the GDT provided is valid and will remain
    /// valid for the entire lifetime of the kernel.
    pub unsafe fn load(&self) {
        x86_64::instr::lgdt(core::ptr::from_ref::<Self>(self) as usize);
    }
}

/// A GDT entry. It is a simple wrapper around a 64-bits integer that
/// represents an entry in the GDT. I don't think it is necessary to
/// provide a constructor for an entry since the GDT is very static
/// and is almost always the same across all operating systems. We can
/// simply use 'magic numbers' to represent the entries in the GDT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Entry(u64);

/// The Global Descriptor Table (GDT) used by the kernel. It is a very standard
/// GDT that looks the same across most operating systems. It contains the
/// following entries:
/// 1. Null entry
/// 2. 64-bit kernel code segment
/// 3. 64-bit kernel data segment
/// 4. 64-bit user data segment
/// 5. 64-bit user code segment
/// 6. Unused
/// 7. TSS entry
/// 8. TSS entry
///
/// The disposition of the entries must not be changed as it is expected by
/// the rest of the kernel, and especially by the `syscall` and `sysret`
/// instructions that require an exact layout of the GDT to work properly.
///
/// Each CPU core has its own GDT, so this table is not shared between CPU
/// cores. This allows to have the same identifier for the TSS entry in the
/// GDT for all CPU cores while still having a different TSS for each CPU core.
#[per_cpu]
static TABLE: Spinlock<[Entry; 8]> = Spinlock::new([
    // Null entry
    Entry(0),
    // 64-bit kernel code segment
    Entry(0x00af_9b00_0000_ffff),
    // 64-bit kernel data segment
    Entry(0x00af_9300_0000_ffff),
    // 64-bit user data segment
    Entry(0x00af_f300_0000_ffff),
    // 64-bit user code segment
    Entry(0x00af_fb00_0000_ffff),
    // Unused
    Entry(0),
    // TSS entry
    Entry(0),
    Entry(0),
]);

/// Initializes the GDT on the current CPU core and reloads the
/// default GDT selectors.
///
/// # Safety
/// The caller must call this function only once during the initialization of
/// the kernel and after the per-CPU data has been initialized.
///
/// # Panics
/// This function will panic if the GDT is not exactly 8 entries long, but this
/// should never happen since the GDT is defined as a static array of 8 entries.
#[init]
pub unsafe fn setup() {
    let register = Register::new(TABLE.local().lock().as_array().unwrap());
    register.load();
    reload_selectors();
}

/// Loads the provided Task State Segment (TSS) in the GDT. It will set the TSS
/// entries in the GDT to the provided TSS (the TSS entry needs to be split in
/// two parts since it is 128-bits long).
///
/// # Safety
/// The caller must ensure that the TSS provided will remain valid until the
/// TSS entry in the GDT is removed. Currently, this means that the TSS must
/// remain in memory for the entire lifetime of the kernel. The caller must
/// also ensure that the memory provided is accessible and readable. Failing
/// to meet these requirements will result in undefined behavior, and probably
/// a triple fault and an immediate reboot of the system.
pub(super) unsafe fn load_tss(tss: *const TaskStateSegment) {
    let address = tss.addr() as u64;
    let mut low = 0;

    // Set the limit to the size of the TSS minus 1 (inclusive limit)
    low.set_bit_range(15, 0, (core::mem::size_of::<TaskStateSegment>() - 1) as u64);

    // Set the low 32 bits of the base address
    low.set_bit_range(63, 56, (address >> 24) & 0xFF);
    low.set_bit_range(39, 16, address & 0xFF_FFFF);

    // Set the type to 0b1001 (x86_64 available TSS)
    low.set_bit_range(43, 40, 0b1001);

    // Set the present bit to 1
    low.set_bit(47, true);

    // Update the TSS entry in the GDT
    TABLE.local().lock()[6] = Entry(low);
    TABLE.local().lock()[7] = Entry(address >> 32);
}

/// Reloads the default GDT selectors (code and data segments) into DS, ES,
/// SS and CS for the current CPU core.
///
/// # Safety
/// The caller must ensure that the GDT is properly loaded and has the expected
/// layout required by the rest of the kernel: the second entry is the 64-bits
/// kernel code segment and the third entry is the 64-bits kernel data segment.
unsafe fn reload_selectors() {
    core::arch::asm!(
        "mov ax, {0}",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "push {1}",
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",
        "2:",
        const Selector::KERNEL_DATA.value(),
        const Selector::KERNEL_CODE.value(),
        lateout("rax") _,
    );
}
