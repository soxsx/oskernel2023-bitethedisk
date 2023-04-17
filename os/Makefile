##################################################################
#                         Configuration                          #
##################################################################
TARGET := riscv64gc-unknown-none-elf
BOOTLOADER = ./bootloader/rustsbi-qemu

KERNEL_ELF := target/$(TARGET)/release/os
KERNEL_ELF_DBG := target/$(TARGET)/debug/os

QEMU = qemu-system-riscv64
GDB = riscv64-unknown-elf-gdb

FAT32_IMAGE = ../workspace/fat32.img
NCPU = 2

##################################################################
#                        Run straightly                          #
##################################################################
initproc:
	@echo Build initproc...
	@cd ../misc && make initproc 

# 不再自动清除缓存，需要手动清除
kernel: initproc
	@cargo build --release

fat32img:
	@echo Build all syscall tests...
	@cd ../misc && make tests 

	@echo Move all tests to workspace...
	@cd ../workspace && mkdir -p tests/
	@cd ../misc && cp -r ./user/build/riscv64/* ../workspace/tests

	@echo Build a fat32 image sized 300MiB...
	@cd ../workspace && make image

run: kernel fat32img
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF) \
		-m 128M \
		-nographic \
		-smp $(NCPU) \
		-bios $(BOOTLOADER) \
		-drive file=$(FAT32_IMAGE),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

##################################################################
#                      Debug build & run.                        #
##################################################################
kernel-debug:
	@cargo clean
	@cargo build

debug-server: kernel-debug fat32img
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF_DBG) \
		-m 128M \
		-nographic \
		-smp $(NCPU) \
		-bios $(BOOTLOADER) \
		-drive file=$(FAT32_IMAGE),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S

debug:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF_DBG)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'
