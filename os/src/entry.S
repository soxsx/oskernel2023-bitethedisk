# 这里需要开启相应拓展以使用 mul
#
# Reference:
# <https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md#-attribute>
.attribute arch, "rv64gc"

.section .text.entry
.global _entry
_entry:
    # 由于我们使用了 SBI，所以这段汇编代码是在 S 特权级运行的，
    # S 特权级不允许我们访问 mhartid，但 SBI 已提前将 mhartid 写入了 a0 寄存器，
    # 所以我们可以通过对 a0 的访问来获取当前的 hartid。
    #
    # Reference:
    # <https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc#function-hart-start-fid-0>

    mv tp, a0      # 将 a0 传过来的 hartid 暂存到 tp
    la sp, stack0
    li a0, 0x2000  # 每个 hart 的栈大小，这里设置为 8KiB
                   # 参考 linux 8KiB 的内核栈
    addi a1, tp, 1
    mul a0, a0, a1 # 为每个 hart 的栈选定起始地址
    add sp, sp, a0 # sp = sp + a0

    call meow
spin:
    j spin

.section .bss.stack
stack0:
    .space 0x2000 * 2  # 根据 -smp 参数，我们将有两个 CPU 可用
