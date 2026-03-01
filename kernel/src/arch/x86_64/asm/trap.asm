.align 16
.global interrupt_handlers
interrupt_handlers:
.set i, 0
.rept 256
# Exceptions with error code.
.if i == 8 || (10 <= i && i <= 14) || i == 17 || i == 21 || i == 29 || i == 30
    .align 16
    push i
    jmp interrupt_enter
# Interrupts and exceptions without error code.
.else
    .align 16
    push 0 # Dummy value as error code.
    push i
    jmp interrupt_enter
.endif

# Increment the counter.
.set i, i + 1
.endr

.align 16
interrupt_enter:
     # Swap the kernel and user GS if we were in user mode
    cmp QWORD ptr [rsp + 8 * 3], 0x08
    je 1f
    swapgs

1:
    # Save scratch registers
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rax

    # Save preserved registers
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp
    
    # Align the stack to 16 bytes and move the pointer 
    # to the pushed registers in RDI
    mov rdi, rsp
    sub rsp, 8

    call trap_handler

    # Dealign the stack
    add rsp, 8

    # Restore preserved registers
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15

    # Restore scratch registers
    pop rax
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11

    # Restore user GS if we were in user mode
    cmp QWORD ptr [rsp + 8 * 3], 0x08
    je 1f
    swapgs

1:
    # Skip error code and interrupt number on the stack
    # and resume execution
    add rsp, 16
    iretq
