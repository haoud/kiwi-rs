OUTPUT_FORMAT("elf64-x86-64")
ENTRY(start)

KERNEL_VIRT_BASE = 0xFFFFFFFF80000000;

SECTIONS
{
    . = KERNEL_VIRT_BASE;
    .text :
    {
        *(.text*)
    }

    . = ALIGN(4096);
    .data :
    {
        *(.data*)
    }

    . = ALIGN(4096);
    .rodata :
    {
        *(.rodata*)
    }

    . = ALIGN(4096);
    .bss (NOLOAD) :
    {
        *(.bss*)
    }

    . = ALIGN(4096);
    _end = .;
    
    /DISCARD/ :
    {
        *(.fini_array*)
        *(.eh_frame*)
        *(.comment)
        *(.note*)
    }
}