BOOTLOADER_ELF = ./kernel/bootloader/rustsbi-qemu
KERNEL_ELF = ./kernel/target/riscv64gc-unknown-none-elf/release/kernel

sbi-qemu:
	@echo Prepare sbi-qemu...
	cp $(BOOTLOADER_ELF) sbi-qemu

kernel-qemu:
	@echo Prepare kernel-qemu...
	cd kernel/ && make kernel
	cp $(KERNEL_ELF) kernel-qemu

all: sbi-qemu kernel-qemu
	@echo Make all finished.

clean:
	rm -f kernel-qemu
	rm -f sbi-qemu
	rm -rf build/
	rm -rf temp/
	cd kernel/ && cargo clean
	cd workspace/ && make clean
	cd fat32/ && cargo clean
	cd misc/ && make clean
	@echo Done.

fat32img:
	cd kernel/ && make fat32img

run:
	cd kernel/ && make run

debug-server:
	cd kernel/ && make debug-server

debug:
	cd kernel/ && make debug
