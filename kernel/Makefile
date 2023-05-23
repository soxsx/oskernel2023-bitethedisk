##################################################################
#                         Configuration                          #
##################################################################
TARGET := riscv64gc-unknown-none-elf
BOOTLOADER = ./bootloader/rustsbi-qemu

KERNEL_ELF := target/$(TARGET)/release/kernel
KERNEL_ELF_DBG := target/$(TARGET)/debug/kernel

QEMU = qemu-system-riscv64
GDB = riscv64-unknown-elf-gdb

FAT32_IMAGE = ../workspace/fat32.img
NCPU = 2

##################################################################
#                        Run straightly                          #
##################################################################
initproc:
	@cd ../misc && make initproc

kernel: initproc
	@cargo build --release --offline

kernel-dbg: initproc
	@LOG_LV=debug cargo build --release --offline

fat32img:
	@cd ../misc && make tests

	@cd ../workspace && mkdir -p tests/
	@cd ../misc && cp -r ./user/build/riscv64/* ../workspace/tests

	@cd ../workspace && make image

run: kernel-dbg fat32img
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
