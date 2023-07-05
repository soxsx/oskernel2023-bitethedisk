.align 3
.section .data
.globl initproc_entry
.globl initproc_tail
initproc_entry:
	.incbin "../crates/libd/target/riscv64gc-unknown-none-elf/release/initproc"
initproc_tail:
