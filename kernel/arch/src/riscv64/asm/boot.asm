.section .init
.globl _start
.align 4
_start:
  la sp, boot_stack_top
  call entry

# Reserve 8 KiB for the boot stack
.section .bss
boot_stack_bottom:
.space 8192
boot_stack_top:
