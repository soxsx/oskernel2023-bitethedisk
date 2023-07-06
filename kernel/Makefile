##################################################################
#                         Configuration                          #
##################################################################
TARGET := riscv64gc-unknown-none-elf
BOOTLOADER = ./bootloader/rustsbi-qemu

KERNEL_ELF := target/$(TARGET)/release/kernel
KERNEL_ELF_DBG := target/$(TARGET)/debug/kernel

QEMU = qemu-system-riscv64
GDB = riscv64-unknown-elf-gdb

SDCARD = ../workspace/sdcard.img
SDCARD_BRK = ../workspace/sdcard.img.brk 
NCPU = 2

RUST_LOG := info

##################################################################
#                        Run straightly                          #
##################################################################
initproc:
	@cd ../crates/libd && cargo build --release

kernel: initproc
	@cargo build --release --offline

kernel-dbg: initproc
	@RUST_LOG=$(RUST_LOG) cargo build --release --offline

sdcard:
	@rm -f $(SDCARD)
	@cp $(SDCARD_BRK) $(SDCARD)

run: kernel-dbg sdcard
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF) \
		-m 128M \
		-nographic \
		-smp $(NCPU) \
		-bios $(BOOTLOADER) \
		-drive file=$(SDCARD),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

##################################################################
#                      Debug build & run.                        #
##################################################################
kernel-debug: initproc
	@RUST_LOG=$(RUST_LOG) cargo build

debug-server: kernel-debug sdcard
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF_DBG) \
		-m 128M \
		-nographic \
		-smp $(NCPU) \
		-bios $(BOOTLOADER) \
		-drive file=$(SDCARD),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S

debug:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF_DBG)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'
