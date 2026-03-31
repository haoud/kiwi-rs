use crate::{
    arch::addr::{Kernel, Virtual},
    library::lock::seq::Seqlock,
};

/// The base address of the IOAPIC MMIO
pub const MMIO_BASE: Virtual<Kernel> = Virtual::<Kernel>::new(0xFFFF_8000_FEC0_0000);

/// The base IRQ number for the IOAPIC
pub const IRQ_BASE: u8 = 32;

/// A register in the IOAPIC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(u32);

impl Register {
    pub const ID: Register = Register(0x00);
    pub const VERSION: Register = Register(0x01);
    pub const ARBITRATION_ID: Register = Register(0x02);
    pub const REDIRECTION_TABLE_BASE: Register = Register(0x10);

    /// Compute the register address for the given redirection entry.
    /// The result will always be used to read the low 32 bits of the entry.
    #[must_use]
    pub const fn redirection_low(n: u8) -> Register {
        Register(Self::REDIRECTION_TABLE_BASE.0 + (n as u32) * 2)
    }

    /// Compute the register address for the given redirection entry.
    /// The result will always be used to read the high 32 bits of the entry.
    #[must_use]
    pub const fn redirection_high(n: u8) -> Register {
        Register(Self::REDIRECTION_TABLE_BASE.0 + (n as u32) * 2 + 1)
    }
}

/// The number of IRQs in the IOAPIC
pub static IRQ_COUNT: Seqlock<u8> = Seqlock::new(0);

/// Setup the I/O APIC and disable all interrupts until they are explicitly
/// enabled.
///
/// # Safety
/// This function must only be called once during the kernel initialization,
/// and the caller must ensure that the APIC and the LAPIC are properly
/// initialized and mapped before calling this function.
pub unsafe fn setup() {
    let irq_count = (read_register(Register::VERSION) >> 16) + 1;
    IRQ_COUNT.write((irq_count & 0xFF) as u8);

    log::debug!("IOAPIC: {} entries found", IRQ_COUNT.read());

    // Disable all interrupts
    for i in 0..IRQ_COUNT.read() {
        write_register(Register::redirection_high(i), 0);
        write_register(Register::redirection_low(i), 1 << 16);
    }
}

/// Enable an IRQ in the IOAPIC, identified by its vector.
///
/// If the IRQ is not handled by the IOAPIC, this function will log a warning
/// and return without enabling the IRQ.
///
/// # Safety
/// This function is unsafe because enabling an IRQ can cause undefined
/// behavior if the IOAPIC is not properly initialized and mapped, or if
/// the IDT handler is misconfigured. The caller must ensure that enabling
/// the IRQ is safe and will not cause undefined behavior.
pub unsafe fn enable_irq(vector: u8) {
    if !own_irq(vector) {
        log::warn!("IOAPIC: Trying to enable IRQ {vector} not owned by the IOAPIC");
        return;
    }

    let irq = vector - IRQ_BASE;
    log::debug!("IOAPIC: Enabling IRQ {irq} with vector {vector}");

    // Enable the IRQ by setting the vector and unmasking it
    write_register(Register::redirection_high(irq), 0);
    write_register(Register::redirection_low(irq), u32::from(vector));
}

/// Disable an IRQ in the IOAPIC, identified by its vector.
///
/// If the IRQ is not handled by the IOAPIC, this function will log a warning
/// and return without disabling the IRQ.
///
/// # Safety
/// This function is unsafe because disabling an IRQ can cause undefined
/// behavior if the IOAPIC is not properly initialized and mapped. The caller
/// must ensure that disabling the IRQ is safe and will not cause undefined
/// behavior.
pub unsafe fn disable_irq(vector: u8) {
    if !own_irq(vector) {
        log::warn!("IOAPIC: Trying to disable IRQ {vector} not owned by the IOAPIC");
        return;
    }

    // Disable the IRQ by masking it
    let irq = vector - IRQ_BASE;
    write_register(Register::redirection_high(irq), 0);
    write_register(Register::redirection_low(irq), 1 << 16);
}

/// Check if an interrupt is owned by the I/O APIC.
///
/// If the I/O APIC is not initialized, the behavior of this function
/// is undefined.
#[must_use]
pub fn own_irq(vector: u8) -> bool {
    (IRQ_BASE..IRQ_BASE + IRQ_COUNT.read()).contains(&vector)
}

/// Write a value to a register in the I/O APIC.
///
/// # Safety
/// This function is unsafe because it writes to a memory-mapped I/O register
/// that controls the I/O APIC.  This could cause unexpected side effects
/// depending on the register being written to, and could lead to undefined
/// behavior or memory unsafety. The caller must ensure that writing to the
/// given register is valid and will not cause any undefined behavior or
/// memory unsafety in the current context.
pub unsafe fn write_register(reg: Register, value: u32) {
    // Tell IOREGSEL what register we want to write to
    // Then write the value to IOWIN
    MMIO_BASE.as_mut_ptr::<u32>().write_volatile(reg.0);
    MMIO_BASE
        .as_mut_ptr::<u32>()
        .byte_add(0x10)
        .write_volatile(value);
}

/// Read a register from an I/O APIC register.
///
/// # Safety
/// This function is unsafe because it reads from a memory-mapped I/O register
/// that controls the I/O APIC.  This could cause unexpected side effects
/// depending on the register being read from, and could lead to undefined
/// behavior or memory unsafety. The caller must ensure that reading from the
/// given register is valid and will not cause any undefined behavior or
/// memory unsafety in the current context.
#[must_use]
pub unsafe fn read_register(reg: Register) -> u32 {
    // Tell IOREGSEL what register we want to read from
    // Then read the value from IOWIN
    MMIO_BASE.as_mut_ptr::<u32>().write_volatile(reg.0);
    MMIO_BASE.as_ptr::<u32>().byte_add(0x10).read_volatile()
}
