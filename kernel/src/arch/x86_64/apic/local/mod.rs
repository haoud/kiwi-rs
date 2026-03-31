use macros::init;

use crate::arch::addr::{Kernel, Virtual};

pub mod timer;

/// The virtual base address of the local APIC registers.
pub const LAPIC_BASE: Virtual<Kernel> = Virtual::<Kernel>::new(0xFFFF_8000_FEE0_0000);

/// A local APIC register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Register(usize);

impl Register {
    pub const END_OF_INTERRUPT: Register = Register(0xB0);
    pub const SPURIOUS_INTERRUPT_VECTOR: Register = Register(0xF0);
    pub const INTERRUPT_COMMAND0: Register = Register(0x300);
    pub const INTERRUPT_COMMAND1: Register = Register(0x310);

    pub const LVT_TIMER: Register = Register(0x320);
    pub const LVT_THERMAL_SENSOR: Register = Register(0x330);
    pub const LVT_PERFORMANCE_MONITORING_COUNTERS: Register = Register(0x340);
    pub const LVT_LINT0: Register = Register(0x350);
    pub const LVT_LINT1: Register = Register(0x360);
    pub const LVT_ERROR: Register = Register(0x370);

    pub const INITIAL_COUNT: Register = Register(0x380);
    pub const CURRENT_COUNT: Register = Register(0x390);
    pub const DIVIDE_CONFIGURATION: Register = Register(0x3E0);
}

/// An inter-processor interrupt (IPI) destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IpiDestination {
    /// Send the IPI to all cores, including the current core.
    All,

    /// Send the IPI to all cores, excluding the current core.
    Others,

    /// Send the IPI to the current core.
    Current,

    /// Send the IPI to a specific core identified by its APIC ID.
    Core(u8),
}

/// An inter-processor interrupt (IPI) priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IpiPriority {
    /// The IPI will be treated as a fixed priority interrupt, and will be
    /// delivered to the destination core(s) according to the fixed priority
    /// scheme.
    Fixed = 0,

    /// The IPI will be treated as a low priority interrupt, and will be
    /// delivered to the least busy core among the destination core(s)
    /// according to the lowest priority scheme.
    Low = 1,

    /// The IPI will be treated as a system management interrupt (SMI). This
    /// should never be used by the kernel.
    SMI = 2,

    /// The IPI will be treated as a non-maskable interrupt (NMI), ignoring
    /// the specified vector and bypassing the normal interrupt handling
    /// mechanisms (cannot be masked and will always be delivered to the
    /// destination core(s) immediately).
    NMI = 4,
}

impl From<IpiPriority> for u32 {
    fn from(value: IpiPriority) -> Self {
        value as u32
    }
}

/// Setup the local APIC subsystem.
///
/// # Safety
/// This function must only be called once per core, and only during the kernel
/// initialization phase.
#[init]
pub unsafe fn setup() {
    // Enable the LAPIC by setting the spurious interrupt vector register's
    // 8th bit to 1, and setting the vector to 0xFF.
    write_register(Register::SPURIOUS_INTERRUPT_VECTOR, 0x1FF);
}

/// Send an Inter-Processor Interrupt (IPI) to the given destination with the
/// given priority and vector (the interrupt number to trigger).
///
/// An IPI is a special type of interrupt that can be sent from one core to
/// another core(s), and is used for various purposes such as inter-core
/// communication, scheduling, and synchronization.
pub fn send_ipi(dst: IpiDestination, priority: IpiPriority, vector: u8) {
    let cmd = match dst {
        IpiDestination::Current => (0, u32::from(vector) | (u32::from(priority) << 8) | 1 << 18),
        IpiDestination::Others => (0, u32::from(vector) | (u32::from(priority) << 8) | 3 << 18),
        IpiDestination::All => (0, u32::from(vector) | (u32::from(priority) << 8) | 2 << 18),
        IpiDestination::Core(core) => (
            u32::from(core) << 24,
            u32::from(vector) | u32::from(priority) << 8,
        ),
    };

    // SAFETY: Sending an IPI is a well-defined operation that should not have
    // any side effects that could lead to undefined behavior or memory
    // unsafety. If triggering an IPI on a core could cause undefined behavior
    // or memory unsafety, this means that the code that would be interrupted
    // by the IPI is unsound and should be fixed.
    unsafe {
        write_register(Register::INTERRUPT_COMMAND1, cmd.0);
        write_register(Register::INTERRUPT_COMMAND0, cmd.1);

        // Wait for the IPI to be sent by polling the delivery status bit
        // (12th bit) of the interrupt command register 0 until it is cleared.
        // TODO: Timeout
        while read_register(Register::INTERRUPT_COMMAND0) & (1 << 12) != 0 {
            core::hint::spin_loop();
        }
    }
}

/// Signal the end of an interrupt (EOI) to the local APIC.
///
/// When a interrupt is triggered by the local APIC, it will not trigger
/// another interrupt until the current interrupt is acknowledged by sending
/// an EOI signal. This function should be called at the end of an interrupt
/// handler to allow the local APIC to trigger the next interrupt.
///
/// Sending an EOI signal is a well-defined operation that does not have any
/// side effects other than allowing the local APIC to trigger the next
/// interrupt. However, even if an interrupt is pending, it will not be
/// delivered until the IF flag is set, so this function can be safely
/// called even if interrupts are currently disabled without risking undefined
/// behavior or memory unsafety.
pub fn signal_eoi() {
    // SAFETY: See the function documentation for details on why this is safe.
    unsafe { write_register(Register::END_OF_INTERRUPT, 0) }
}

/// Writes a 32-bit value to the given register.
///
/// # Safety
/// This function is unsafe because it writes to a memory-mapped I/O register
/// that controls the local APIC.  This could cause unexpected side effects
/// depending on the register being written to, and could lead to undefined
/// behavior or memory unsafety. The caller must ensure that writing to the
/// given register is valid and will not cause any undefined behavior or
/// memory unsafety in the current context.
pub unsafe fn write_register(register: Register, value: u32) {
    LAPIC_BASE
        .as_mut_ptr::<u32>()
        .byte_add(register.0)
        .write_volatile(value);
}

/// Reads a 32-bit value from the given register.
///
/// # Safety
/// This function is unsafe because it reads from a memory-mapped I/O register
/// that controls the local APIC.  This could cause unexpected side effects
/// depending on the register being read from, and could lead to undefined
/// behavior or memory unsafety. The caller must ensure that reading from the
/// given register is valid and will not cause any undefined behavior or
/// memory unsafety in the current context.
#[must_use]
pub unsafe fn read_register(register: Register) -> u32 {
    LAPIC_BASE
        .as_ptr::<u32>()
        .byte_add(register.0)
        .read_volatile()
}
