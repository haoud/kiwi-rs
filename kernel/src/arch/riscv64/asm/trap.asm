.section .text
.extern thread_trap
.globl kernel_enter
.align 4
kernel_enter:
  # Save the user's stack pointer and set the kernel stack
  # atomically using the sscratch register
  csrrw sp, sscratch, sp

  # Check if were in user mode or in kernel mode and jump
  # to the appropriate trap handler
  beqz sp, kernel_trap
  bnez sp, thread_trap

.align 4
kernel_trap:
  # Restore the kernel stack pointer that was swapped with
  # the ssratch register and save all registers into the
  # kernel stack
  csrrw sp, sscratch, sp
  addi sp, sp, -32*8
  sd x1, 0*8(sp)
  sd x2, 1*8(sp)
  sd x3, 2*8(sp)
  sd x4, 3*8(sp)
  sd x5, 4*8(sp)
  sd x6, 5*8(sp)
  sd x7, 6*8(sp)
  sd x8, 7*8(sp)
  sd x9, 8*8(sp)
  sd x10, 9*8(sp)
  sd x11, 10*8(sp)
  sd x12, 11*8(sp)
  sd x13, 12*8(sp)
  sd x14, 13*8(sp)
  sd x15, 14*8(sp)
  sd x16, 15*8(sp)
  sd x17, 16*8(sp)
  sd x18, 17*8(sp)
  sd x19, 18*8(sp)
  sd x20, 19*8(sp)
  sd x21, 20*8(sp)
  sd x22, 21*8(sp)
  sd x23, 22*8(sp)
  sd x24, 23*8(sp)
  sd x25, 24*8(sp)
  sd x26, 25*8(sp)
  sd x27, 26*8(sp)
  sd x28, 27*8(sp)
  sd x29, 28*8(sp)
  sd x30, 29*8(sp)
  sd x31, 30*8(sp)

  call kernel_trap_handler

  # Restore all registers
  ld x1, 0*8(sp)
  ld x2, 1*8(sp)
  ld x3, 2*8(sp)
  ld x4, 3*8(sp)
  ld x5, 4*8(sp)
  ld x6, 5*8(sp)
  ld x7, 6*8(sp)
  ld x8, 7*8(sp)
  ld x9, 8*8(sp)
  ld x10, 9*8(sp)
  ld x11, 10*8(sp)
  ld x12, 11*8(sp)
  ld x13, 12*8(sp)
  ld x14, 13*8(sp)
  ld x15, 14*8(sp)
  ld x16, 15*8(sp)
  ld x17, 16*8(sp)
  ld x18, 17*8(sp)
  ld x19, 18*8(sp)
  ld x20, 19*8(sp)
  ld x21, 20*8(sp)
  ld x22, 21*8(sp)
  ld x23, 22*8(sp)
  ld x24, 23*8(sp)
  ld x25, 24*8(sp)
  ld x26, 25*8(sp)
  ld x27, 26*8(sp)
  ld x28, 27*8(sp)
  ld x29, 28*8(sp)
  ld x30, 29*8(sp)
  ld x31, 30*8(sp)
  addi sp, sp, 32*8
  sret
