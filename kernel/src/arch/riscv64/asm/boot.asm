.equ KERNEL_VIRTUAL_BASE, 0xFFFFFFFFC0000000
.equ KERNEL_PHYSICAL_BASE, 0x80000000

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

  # Disable interrupts
  csrw sie, zero
  csrc sip, zero

  # Setup satp with sv39 mode and the boot page table
  la t0, boot_page_table
  srli t0, t0, 12
  li t1, 8
  slli t1, t1, 60
  or t0, t0, t1
  csrw satp, t0

  # Clear the BSS section with zeros
	LA_FAR t2, __bss_start
  LA_FAR t3, __bss_end
.bss_clear:
	sd zero, (t2)
	addi t2, t2, 8
	bltu t2, t3, .bss_clear

  # Update the pointer in a1 to use the kernel virtual base
  la t0, KERNEL_PHYSICAL_BASE
  la t1, KERNEL_VIRTUAL_BASE
  sub a1, a1, t0
  add a1, a1, t1

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

# Reserve 64 KiB for the boot stack
.section .bss
boot_stack_bottom:
.space 64 * 1024
boot_stack_top:

