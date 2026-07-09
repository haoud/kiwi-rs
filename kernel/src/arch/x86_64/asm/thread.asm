
# Called when a kernel thread wants to exit. This function will fake a trap to
# the kernel, similar to what happens when a kernel thread is interrupted, but
# will not save the context of the kernel thread since it is exiting and there
# is no need to save the context of the thread.
.global thread_exit
.align 16
thread_exit:
    # Disable interrupts
    cli

    # Clear rax to return a null pointer to the caller of thread_resume since
    # the thread has exited and there is no context to save. Also clear rbx
    # to clear the per-cpu variable that stores the stack pointer of the caller
    # to indicate that the kernel is no longer executing a kernel thread.
    xor rax, rax
    xor rbx, rbx
    jmp 1f


# Called when a kernel thread is interrupted. This function should be called
# from the interrupt handler with the saved context of the kernel thread in 
# RDI. This function will restore the context of the caller of thread_resume
# and return to it, allowing the kernel to process the interrupt.
.global thread_trap
.align 16
thread_trap:
    mov rax, rsp
    xor rbx, rbx

    # Restore the stack pointer of the caller and clear the per-cpu variable
    # that stores the stack pointer of the caller to indicate that the kernel is
    # no longer executing a user/kernel thread.
1:
    mov rsp, gs:0x08
    mov gs:0x08, rbx

    # Restore the registers that must be preserved across function calls
    # as well as rflags and return to the caller of the function that was
    # interrupted.
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15
    popf
    ret

# Resume the execution of a kernel/user thread that was interrupted. This 
# function saves the context of the caller and resumes the execution of the
# thread whose context is passed in RDI. When the thread is interrupted, the
# context of the thread will be saved and this function will return normally
# to the caller of thread_resume, returning in RAX the saved interrupt frame
# of the thread.
.global thread_resume
.align 16
thread_resume:
    # Save registers that must be preserved across function calls, as well as
    # the rflags register
    pushf
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp

    # Disable interrupts
    cli

    # Save the stack pointer in a per-cpu variable to be able to return to the
    # caller of this function when a trap occurs.
    mov gs:0x08, rsp

    # Change the stack pointer to the interrupt frame of the thread
    # in order to resume the execution of the thread
    mov rsp, rdi
    jmp trap_resume
