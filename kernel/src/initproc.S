.align 3
.section .data
.globl initproc_entry
.globl initproc_tail
initproc_entry:
	.incbin "../misc/tests_booter/bin/tests_booter"
initproc_tail:
