# Parameters:
#   a0: pointer to the thread's context
.section .text 
.align 4
thread_execute:
  # Allocate 15 words on the kernel stack to save registers that must
  # be preserved across function calls by the RISC-V ABI since those
  # registers will be used by the user code.
  addi sp, sp, -16 * 8
  sd ra, 0(sp)
  sd gp, 8(sp)
  sd tp, 16(sp)
  sd s0, 24(sp)
  sd s1, 32(sp)
  sd s2, 40(sp)
  sd s3, 48(sp)
  sd s4, 56(sp)
  sd s5, 64(sp)
  sd s6, 72(sp)
  sd s7, 80(sp)
  sd s8, 88(sp)
  sd s9, 96(sp)
  sd s10, 104(sp)
  sd s11, 112(sp)
  
  # Store the thread's context into the ssratch register
  csrw sscratch, a0

  # Store the kernel stack pointer into the thread's context
  # padding word. This allow the trap handler to easily restore
  # the kernel stack pointer when the thread traps.
  sd sp, 264(a0)
 
  # Set the stack pointer to the thread's context
  mv sp, a0

  # Load and restore user sepc
  ld t0, 256(sp)
  csrw sepc, t0

  # Load and restore user sstatus
  ld t0, 248(sp)
  csrw sstatus, t0

  # Restore the thread's context
  ld x1, 0(sp)
  ld x3, 16(sp)
  ld x4, 24(sp)
  ld x5, 32(sp)
  ld x6, 40(sp)
  ld x7, 48(sp)
  ld x8, 56(sp)
  ld x9, 64(sp)
  ld x10, 72(sp)
  ld x11, 80(sp)
  ld x12, 88(sp)
  ld x13, 96(sp)
  ld x14, 104(sp)
  ld x15, 112(sp)
  ld x16, 120(sp)
  ld x17, 128(sp)
  ld x18, 136(sp)
  ld x19, 144(sp)
  ld x20, 152(sp)
  ld x21, 160(sp)
  ld x22, 168(sp)
  ld x23, 176(sp)
  ld x24, 184(sp)
  ld x25, 192(sp)
  ld x26, 200(sp)
  ld x27, 208(sp)
  ld x28, 216(sp)
  ld x29, 224(sp)
  ld x30, 232(sp)
  ld x31, 240(sp)

  # Restore the stack pointer and return to the user code
  ld sp, 8(sp)
  sret

# Thread that traps will return here. This function will restore the kernel state
# and then return to code that called `thread_execute`.
.align 4
thread_trap:
  # Save the thread's context
  sd x1, 0(sp)
  sd x3, 16(sp)
  sd x4, 24(sp)
  sd x5, 32(sp)
  sd x6, 40(sp)
  sd x7, 48(sp)
  sd x8, 56(sp)
  sd x9, 64(sp)
  sd x10, 72(sp)
  sd x11, 80(sp)
  sd x12, 88(sp)
  sd x13, 96(sp)
  sd x14, 104(sp)
  sd x15, 112(sp)
  sd x16, 120(sp)
  sd x17, 128(sp)
  sd x18, 136(sp)
  sd x19, 144(sp)
  sd x20, 152(sp)
  sd x21, 160(sp)
  sd x22, 168(sp)
  sd x23, 176(sp)
  sd x24, 184(sp)
  sd x25, 192(sp)
  sd x26, 200(sp)
  sd x27, 208(sp)
  sd x28, 216(sp)
  sd x29, 224(sp)
  sd x30, 232(sp)
  sd x31, 240(sp)

  # Save the user stack pointer and clear the ssratch register
  csrrw t0, sscratch, zero 
  sd t0, 8(sp)

  # Save the user sstatus
  csrr t0, sstatus
  sd t0, 248(sp)

  # Save the user sepc
  csrr t0, sepc
  sd t0, 256(sp)

  # Switch to the kernel stack
  ld sp, 264(sp)

  # Restore the kernel state
  ld ra, 0(sp)
  ld gp, 8(sp)
  ld tp, 16(sp)
  ld s0, 24(sp)
  ld s1, 32(sp)
  ld s2, 40(sp)
  ld s3, 48(sp)
  ld s4, 56(sp)
  ld s5, 64(sp)
  ld s6, 72(sp)
  ld s7, 80(sp)
  ld s8, 88(sp)
  ld s9, 96(sp)
  ld s10, 104(sp)
  ld s11, 112(sp)
  
  # Go back to the caller of `thread_execute`
  addi sp, sp, 16 * 8
  ret
  
