# Building
TARGET := riscv64imac-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin

BOARD ?= qemu
debug ?= 0

file ?= final/stage2

# Binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64

build: env initproc_submit $(KERNEL_BIN) 

initproc_submit:
	@cd ../user_c && make build_initproc_submit

env:
	rustup target add $(TARGET)
#	cargo install cargo-binutils --vers =0.3.3
#	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
    # @cargo clean
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --offline --features "board_$(BOARD)" --features "debug_$(debug)"
	@rm src/linker.ld

run: build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu.bin \
		-device loader,file=$(KERNEL_BIN),addr=0x80200000 \
		-drive file=../simple-fat32/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

copy-to-fs: 
	@cd ../simple-fat32 \
	&& rm -f ./fat32.img \
	&& sh ./create_img.sh \
	&& sudo rm -rf fs \
	&& sudo mkdir fs \
	&& sudo mount fat32.img fs \
	&& sudo cp -r ../test_program/$(file)/sdcard_content/* fs/ \
	&& sleep 1 \
	&& sudo umount fs \
	&& sudo rmdir fs
	
dump:
	@riscv64-unknown-elf-objdump -S $(KERNEL_ELF) > os_dump

.PHONY: test
test: copy-to-fs 
	@rm -f output
	@make build > output
	@echo "\nTest start! Please wait a minute..."
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu.bin \
		-device loader,file=$(KERNEL_BIN),addr=0x80200000 \
		-drive file=../simple-fat32/fat32.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		> output
	@sh auto_test.sh


.PHONY: dump run submit build-custom-test

.DEFAULT_GOAL = $(KERNEL)

KERNEL = bite_the_disk

NHART = 1 # hart 数
MACHINE = virt # sifive_u 或者 virt
CPU = rv64
MEM = 128M

KERNEL_ELF = target/riscv64gc-unknown-none-elf/release/$(KERNEL)
KERNEL_BIN = target/riscv64gc-unknown-none-elf/release/$(KERNEL).bin

BOOTLOADER = bootloader/rustsbi-qemu.bin

QEMU = qemu-system-riscv64

TOOLPREFIX = riscv64-unknown-elf-

GDB = $(TOOLPREFIX)gdb


LOG_LV = DEBUG

$(KERNEL):
	@LOG_LV=$(LOG_LV) cargo build --release
	@rust-objcopy --strip-all $(KERNEL_ELF) -O binary $(KERNEL_BIN)

.PHONY: run
run: $(KERNEL)
	@$(QEMU) \
		-M $(MACHINE) \
		-serial mon:stdio \
		-nographic \
		-cpu $(CPU) \
		-bios $(BOOTLOADER) \
		-m $(MEM) \
		-kernel $(KERNEL_BIN) \
		-smp $(NHART)

.PHONY: debug-server
debug-server: $(KERNEL)
	@$(QEMU) \
		-M $(MACHINE) \
		-serial mon:stdio \
		-nographic \
		-cpu $(CPU) \
		-bios $(BOOTLOADER) \
		-m $(MEM) \
		-kernel $(KERNEL_BIN) \
		-smp $(NHART) \
		-s -S

.PHONY: debug
debug:
	@$(GDB) \
		-ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'

.PHONY: clean
clean:
	@cargo clean
	@echo [make] all previous build has been cleaned up.
