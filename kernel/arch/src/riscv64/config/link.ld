OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    __init_start = .;
    .init :
    {
        *(.init .init.*)
    }
    . = ALIGN(4K);
    __init_end = .;

    .text :
    {
        *(.text .text.*)
    }
    . = ALIGN(4K);

    .rodata : 
    {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);

    .data : 
    {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);

    __bss_start = .;
    .bss (NOLOAD) : 
    {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    __bss_end = .;

    /DISCARD/ : 
    {
        *(.eh_frame)
    }
}
