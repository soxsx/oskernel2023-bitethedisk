BOOTLOADER_ELF = ./kernel/bootloader/rustsbi-qemu
KERNEL_ELF = ./kernel/target/riscv64gc-unknown-none-elf/release/kernel

sbi-qemu:
	@cp $(BOOTLOADER_ELF) sbi-qemu

kernel-qemu:
	@mv kernel/cargo kernel/.cargo
	@cd kernel/ && make kernel
	@cp $(KERNEL_ELF) kernel-qemu

all: sbi-qemu kernel-qemu

clean:
	@rm -f kernel-qemu
	@rm -f sbi-qemu
	@rm -rf build/
	@rm -rf temp/
	@cd kernel/ && cargo clean
	@cd workspace/ && make clean
	@cd crates/libd && cargo clean
	@cd crates/sync_cell && cargo clean
	@cd crates/fat32 && cargo clean
	@cd testsuits/ && make clean

run:
	@cd kernel/ && make run

debug-server: check-sdcard
	@cd kernel/ && make debug-server

debug: 
	@cd kernel/ && make debug

sdcard:
	@cd testsuits/ \
	&& docker run --rm -it -v $$(pwd):/code --privileged --entrypoint make alphamj/os-contest:v7.7 -C /code sdcard \
	&& mv sdcard.img ../workspace/sdcard.img.bak
