.macro LA_FAR, reg, sym
	lui \reg, %hi(\sym)
	addi \reg, \reg, %lo(\sym)
.endm

.section .early, "ax"
.globl _start
.align 4
_start:
  .option push
  .option norelax

  # Setup satp with sv39 mode and the boot page table
  la t0, boot_page_table
  srli t0, t0, 12
  li t1, 8
  slli t1, t1, 60
  or t0, t0, t1
  csrw satp, t0

  # Setup the stack pointer and jump to the entry point
  LA_FAR sp, boot_stack_top
  LA_FAR t0, entry
  jr t0

  .option pop

# The boot page table
.align 12
boot_page_table:
  .quad 0x000000000000000F
  .quad 0x000000001000000F
  .quad 0x000000002000000F
  .fill 508, 8, 0
  .quad 0x000000002000000F

# Reserve 8 KiB for the boot stack
.section .bss
boot_stack_bottom:
.space 8192
boot_stack_top:

