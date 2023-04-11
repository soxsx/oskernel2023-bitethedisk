BOOTLOADER_ELF = ./os/bootloader/rustsbi-qemu
KERNEL_ELF = ./os/target/riscv64gc-unknown-none-elf/release/os

sbi-qemu:
	@echo Prepare sbi-qemu...
	cp $(BOOTLOADER_ELF) sbi-qemu

kernel-qemu:
	@echo Prepare kernel-qemu...
	cd os/ && make kernel
	cp $(KERNEL_ELF) kernel-qemu

all: sbi-qemu kernel-qemu
	@echo Make all finished.

clean:
	rm -f kernel-qemu
	rm -f sbi-qemu
	rm -rf build/
	rm -rf temp/
	cd os/ && cargo clean
	cd workspace/ && make clean
	cd fat32/ && cargo clean
	cd misc/user && make clean
	@echo Done.

fat32img:
	cd os/ && make fat32img

run:
	cd os/ && make run

debug-server:
	cd os/ && make debug-server

debug:
	cd os/ && make debug
