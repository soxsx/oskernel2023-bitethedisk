.section .text
.globl __switch
__switch:
        #
        # fn __switch(current_task_cx_ptr: *mut TaskContext,
        #             next_task_cx_ptr:    *const TaskContext);
        #
        # 根据 riscv ABI，函数的入参依次放在寄存器 a0, a1, a2...a7 中
        #
        # a0 -> current_task_ctx_ptr: *mut TaskContext
        # a1 -> next_task_ctx_ptr:    *const TaskContext
        #

        # Save current task context.
        sd ra, 0(a0)
        sd sp, 8(a0)
        sd s0, 16(a0)
        sd s1, 24(a0)
        sd s2, 32(a0)
        sd s3, 40(a0)
        sd s4, 48(a0)
        sd s5, 56(a0)
        sd s6, 64(a0)
        sd s7, 72(a0)
        sd s8, 80(a0)
        sd s9, 88(a0)
        sd s10, 96(a0)
        sd s11, 104(a0)

        # Load the next task context and be ready to run.
        ld ra, 0(a1)
        ld sp, 8(a1)
        ld s0, 16(a1)
        ld s1, 24(a1)
        ld s2, 32(a1)
        ld s3, 40(a1)
        ld s4, 48(a1)
        ld s5, 56(a1)
        ld s6, 64(a1)
        ld s7, 72(a1)
        ld s8, 80(a1)
        ld s9, 88(a1)
        ld s10, 96(a1)
        ld s11, 104(a1)

        ret
