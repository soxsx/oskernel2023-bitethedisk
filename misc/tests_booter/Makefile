ARCH := riscv64

s_DIR := ./src/*.s
LIB_DIR := ./lib/*.c
ifeq ($(ARCH), riscv64)
LINKER_PATH := ./lib/arch/riscv/user.ld
CC := riscv64-unknown-elf-gcc -ffreestanding -nostdlib -g -mcmodel=medany -I ./include -I ./lib/arch/riscv
endif

.PHONY: initproc
initproc:
	rm -rf bin
	mkdir bin
	$(CC) ./src/tests_booter.c $(LIB_DIR) $(s_DIR) -T $(LINKER_PATH) -o ./bin/tests_booter
