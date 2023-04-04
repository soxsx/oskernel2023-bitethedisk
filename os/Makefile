TARGET := riscv64imac-unknown-none-elf

KERNEL_ELF := target/$(TARGET)/release/os
KERNEL_BIN := $(KERNEL_ELF).bin

BOARD ?= qemu

env:
	rustup target add $(TARGET)
	rustup component add llvm-tools-preview

initproc:
	@cd ../misc/user_c && make build

build: env
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --features "board_$(BOARD)"
	@rm src/linker.ld

run: build copy_to_fs
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu \
		-kernel $(KERNEL_ELF) \
		-drive file=../workspace/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

copy_to_fs: 
	cd ../workspace && sudo mount fat32.img fs/ && sudo cp -r ../misc/user_c/bin/* fs/ && sudo umount fs/
