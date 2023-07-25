##################################################################
#                         Configuration                          #
##################################################################
TARGET := riscv64gc-unknown-none-elf

KERNEL_ELF := target/$(TARGET)/release/kernel
KERNEL_ELF_DBG := target/$(TARGET)/debug/kernel

QEMU = qemu-system-riscv64
GDB = riscv64-unknown-elf-gdb

SDCARD = ../workspace/sdcard.img
SDCARD_BAK = ../workspace/sdcard.img.bak
NCPU = 2

RUST_LOG := info

##################################################################
#                        Run straightly                          #
##################################################################
initproc:
	@cd ../crates/libd && cargo build --release
sdcard:
	@rm -f $(SDCARD)
	@cp $(SDCARD_BAK) $(SDCARD)

kernel: initproc
	@RUST_LOG=warn cargo build --release --offline

kernel-dbg: initproc
	@RUST_LOG=$(RUST_LOG) cargo build --release --offline

run: kernel-dbg sdcard
	@$(QEMU) \
	 	-machine virt \
		-kernel $(KERNEL_ELF) \
		-m 128M \
		-nographic \
		-smp $(NCPU) \
		-bios default \
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
		-drive file=$(SDCARD),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S

debug:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF_DBG)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'
