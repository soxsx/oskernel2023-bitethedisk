# 这里需要开启相应拓展以使用 mul
#
# Reference:
# <https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md#-attribute>
.attribute arch, "rv64gc"

.section .text.entry
.globl _entry
_entry:
        mv tp, a0
        la sp, stack0
        li a0, 0x4000  # 16KiB

        addi a1, tp, 1
        mul a0, a0, a1
        add sp, sp, a0

        call meow

spin:
        j spin

.section .bss.stack
stack0:                 
        .space 0x4000 * 2  # -smp 2
