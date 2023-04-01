# 这里需要开启相应拓展以使用 mul
#
# Reference:
# <https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md#-attribute>
.attribute arch, "rv64gc"

.section .text.entry
.global _entry
_entry:
    # 由于我们使用了 SBI，所以这段汇编代码是在 S 特权级运行的
    # 但是 S 特权级不允许我们直接调用 mhartid，但 SBI 已经提前
    # 将 mhartid 写入了 a0 寄存器
    #
    # Reference:
    # <https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc#function-hart-start-fid-0>

    mv tp, a0      # 将 a0 传过来的 hartid 暂存到 tp
    la sp, stack0
    li a0, 0x1000  # 每个 hart 的栈大小，这里设置为 4KiB
    addi a1, tp, 1
    mul a0, a0, a1   # 为每个 hart 的栈选定起始地址
    add sp, sp, a0   # sp = sp + a0

    call meow
spin:
    j spin

.section .bss.stack
stack0:
    .space 0x1000 * 5 # 每个 hart 分配 0x1000 => 4KiB 栈内存

# 有关 riscv 寄存器的信息:
# <https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md>
