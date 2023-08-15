# kernel/task/initproc/initproc.S
#
# All of these below will become a part of the kernel.

.align 3
.section .data
.globl initproc_entry
.globl initproc_tail
initproc_entry:
	.incbin "../crates/libd/target/riscv64gc-unknown-none-elf/release/initproc"
initproc_tail:

.align 3
.section .data
.globl busybox_entry
.globl busybox_tail
busybox_entry:
	.incbin "src/task/initproc/busybox"
busybox_tail:
