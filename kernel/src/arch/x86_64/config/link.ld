OUTPUT_FORMAT("elf64-x86-64")
ENTRY(start)

KERNEL_VIRT_BASE = 0xFFFFFFFF80000000;

SECTIONS
{
    . = KERNEL_VIRT_BASE;

    .init : ALIGN(4K)
    {
        // Per-CPU data for each CPU. Similar to per-cpu constructor, we can put
        // the per-cpu data in the .init section, which will be discarded after
        // the initialization is complete since each CPU will have its own copy
        // of the per-cpu data.
        // We reserve 64 bytes for each CPU at the beginning of the per-cpu data
        // section to store the CPU ID that architecture-specific code can use
        // freely for internal purposes.
        __percpu_start = .;
        . += 64;
        *(.data.percpu*)
        __percpu_end = .;

        // Constructors for initialization of per-CPU data for each CPU. Since
        // per-cpu data are only initialized during the boot process (Kiwi does
        // not support hot-plugging of CPUs), we can put the constructors in the
        // .init section, which will be discarded after the initialization is
        // complete.
        . = ALIGN(8);
        __percpu_ctors_start = .;
        *(.percpu.ctors*)
        __percpu_ctors_end = .;

        *(.init.text*)
        *(.init.data*)
    }

    .text : ALIGN(4K)
    {
        *(.text*)
    }

    .data : ALIGN(4K)
    {

        *(.data*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata*)
    }

    .bss (NOLOAD) : ALIGN(4K)
    {
        *(.bss*)
    }

    _end = .;

    /DISCARD/ :
    {
        *(.fini_array*)
        *(.eh_frame*)
        *(.comment)
        *(.note*)
    }
}
