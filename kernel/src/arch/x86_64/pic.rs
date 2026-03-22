use macros::init;

use crate::arch::x86_64::io::{InlinePort, Write};

/// The base IRQ number for the PICs
pub const IRQ_BASE: u8 = 32;

static MASTER_PIC_CMD: InlinePort<0x20, u8, Write> = InlinePort::new();
static MASTER_PIC_DATA: InlinePort<0x21, u8, Write> = InlinePort::new();
static SLAVE_PIC_CMD: InlinePort<0xA0, u8, Write> = InlinePort::new();
static SLAVE_PIC_DATA: InlinePort<0xA1, u8, Write> = InlinePort::new();

/// Setup the programmable interrupt controllers (PICs) to remap the IRQs to
/// a free range of interrupt numbers (by default, the PICs use interrupt
/// numbers 0-15, which conflicts with the CPU exceptions).
///
/// Kiwi does not support the legacy PICs and instead uses the more modern
/// APICs that are more flexible and powerful. However, the PICs are still
/// present in the system and can cause spurious interrupts if not properly
/// configured. Therefore, we need to remap the PICs to a different range of
/// interrupt numbers and disable all interrupts from them.
///
/// # Safety
/// This function should only be called once during the kernel initialization
/// process, and it should not be called after the APICs have been initialized.
#[init]
pub unsafe fn setup() {
    // ECW1: Cascade mode, ICW4 needed
    MASTER_PIC_CMD.write_and_pause(0x11);
    SLAVE_PIC_CMD.write_and_pause(0x11);

    // ICW2: Write the base IRQs for the PICs
    MASTER_PIC_DATA.write_and_pause(IRQ_BASE);
    SLAVE_PIC_DATA.write_and_pause(IRQ_BASE + 8);

    // ICW3: Connect the PICs to each other
    MASTER_PIC_DATA.write_and_pause(4);
    SLAVE_PIC_DATA.write_and_pause(2);

    // ICW4: Request 8086 mode
    MASTER_PIC_DATA.write_and_pause(0x01);
    SLAVE_PIC_DATA.write_and_pause(0x01);

    // OCW1: Disable all interrupts
    MASTER_PIC_DATA.write_and_pause(0xFF);
    SLAVE_PIC_DATA.write_and_pause(0xFF);
}
