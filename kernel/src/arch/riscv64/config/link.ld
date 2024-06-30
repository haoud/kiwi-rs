OUTPUT_ARCH(riscv)
ENTRY(_start)

KERNEL_VIRTUAL_CODE_BASE = 0xFFFFFFFFC0200000;
KERNEL_VIRTUAL_BASE = 0xFFFFFFFFC0000000;
KERNEL_PHYSICAL_CODE_BASE = 0x80200000;
KERNEL_PHYSICAL_OFFSET = 0x200000;
RAM_START = 0x80000000;

SECTIONS
{
    . = KERNEL_PHYSICAL_CODE_BASE;
    .early :
    {
        __early_start = .;
        *(.early .early.*)
        . = ALIGN(4K);
        __early_end = .;
    }


    OFFSET = __early_end - __early_start;
    . = KERNEL_VIRTUAL_CODE_BASE + OFFSET;

    __reclaimable_start = KERNEL_VIRTUAL_CODE_BASE;
    __start = .;

    .init ALIGN(4K) : AT(ADDR(.init) - KERNEL_VIRTUAL_BASE + RAM_START)
    {
        __init_start = .;
        *(.init .init.*)
        __init_end = .;
    }
    __reclaimable_end = .;

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VIRTUAL_BASE + RAM_START)
    {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VIRTUAL_BASE + RAM_START)
    {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VIRTUAL_BASE + RAM_START)
    {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .bss ALIGN(4K) (NOLOAD) : AT(ADDR(.bss) - KERNEL_VIRTUAL_BASE + RAM_START)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        __bss_end = .;
    }

    __end = .;

    /DISCARD/ : 
    {
        *(.eh_frame)
    }
}
