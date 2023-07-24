# Attributes are used to record information about an object file/binary that a
# linker or runtime loader needs to check compatibility.
#
# Reference:
# <https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md#-attribute>
.attribute arch, "rv64gc"

.equ NCPU, 2            # number of CPU
.equ KSTACK_SIZ, 0x10000 # size of stack size per hart

.section .text.entry
.globl _entry
_entry:
        mv tp, a0
        la sp, stack0
        li a0, KSTACK_SIZ  # 64KiB

        addi a1, tp, 1
        mul a0, a0, a1
        add sp, sp, a0

        call meow

spin:
        j spin

.section .bss.stack
stack0:
        .space KSTACK_SIZ * NCPU  # -smp 2
