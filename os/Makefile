TARGET := riscv64imac-unknown-none-elf

KERNEL_ELF := target/$(TARGET)/release/os
KERNEL_BIN := $(KERNEL_ELF).bin

KERNEL_ELF_DBG := target/$(TARGET)/debug/os

BOARD ?= qemu

env:
	rustup target add $(TARGET)
	rustup component add llvm-tools-preview

initproc:
	@cd ../misc/user_c && make build

build:
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo clean && cargo build --release --features "board_$(BOARD)"
	@rm src/linker.ld

build-debug:
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo clean && cargo build --features "board_$(BOARD)"
	@rm src/linker.ld

run: build copy_to_fs
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-smp 1 \
		-bios ../bootloader/rustsbi-qemu \
		-kernel $(KERNEL_ELF) \
		-drive file=../workspace/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

copy_to_fs: 
	cd ../workspace && sudo mount fat32.img fs/ && sudo cp -r ../misc/user_c/bin/* fs/ && sudo umount fs/

NCPU = 1

QEMU = qemu-system-riscv64
GDB = riscv64-unknown-elf-gdb

.PHONY: debug-server
debug-server: build-debug
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF_DBG) \
		-serial mon:stdio \
		-nographic \
		-bios ../bootloader/rustsbi-qemu \
		-m 128M \
		-smp $(NCPU) \
		-s -S

.PHONY: debug
debug:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF_DBG)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'
