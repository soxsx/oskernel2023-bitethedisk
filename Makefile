# all:
# 	@cd os && make build BOARD=k210
# 	@cp $(BOOTLOADER) $(BOOTLOADER).copy
# 	@dd if=./os/target/riscv64imac-unknown-none-elf/release/os.bin of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
# 	@mv $(BOOTLOADER).copy ./os.bin

gdb-run:
#	@cd os && make build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ./bootloader/rustsbi-qemu.bin \
		-device loader,file=./os/target/riscv64imac-unknown-none-elf/release/os.bin,addr=0x80200000 \
		-drive file=./simple-fat32/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S 

BOOTLOADER_ELF = bootloader/rustsbi-qemu
sbi-qemu:
	@echo Generate sbi-qemu...
	@cp $(BOOTLOADER_ELF) sbi-qemu

KERNEL_ELF = os/target/riscv64imac-unknown-none-elf/release/os
kernel-qemu:
	@echo Build kernel-qemu...
	@cd os/ && make build
	@cp $(KERNEL_ELF) kernel-qemu

.PHONY: all
all: sbi-qemu kernel-qemu
	@echo Build all...

.PHONY: clean
clean:
	@echo Clean all previous build...
	@cd os/ && cargo clean && cd ..
	@cd fat32/ && cargo clean && cd ..
	@rm -rf build
	@rm -f sbi-qemu kernel-qemu

# 区域赛完整的 qemu 命令
# @qemu-system-riscv64 \
# 	-machine virt \
# 	-kernel kernel-qemu \
# 	-m 128M \
# 	-nographic \
# 	-smp 2 \
# 	-bios sbi-qemu \
# 	-drive file=sdcard.img,if=none,format=raw,id=x0 \
# 	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
# 	-initrd initrd.img

# 只用来测试
run:
	cd os/ && make run
