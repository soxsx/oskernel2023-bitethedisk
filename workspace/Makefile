ifeq ($(shell id -u), 0)
  SU =
else
  SU = sudo
endif

image:
	@rm -f fat32.img
	@dd if=/dev/zero of=fat32.img bs=3M count=100
	@echo Format the image to fat32...
	@mkfs.vfat -F 32 fat32.img
	@chmod 777 fat32.img
	@mkdir -p fs/
	@$(SU) mount fat32.img fs/
	@$(SU) cp -r tests/* fs/
	@$(SU) umount fs/
	@$(SU) rm -rf ../misc/user/riscv64

clean:
	rm -f fat32.img
	rm -rf tests/
	umount fs/ ; rm -rf fs/

