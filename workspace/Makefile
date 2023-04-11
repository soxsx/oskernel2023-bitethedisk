image: 
	@rm -f fat32.img 
	@dd if=/dev/zero of=fat32.img bs=3M count=100
	@echo Format the image to fat32...
	mkfs.vfat -F 32 fat32.img
	chmod 777 fat32.img
	mkdir -p fs/
	sudo mount fat32.img fs/
	sudo cp -r tests/* fs/
	sudo umount fs/

clean:
	rm -f fat32.img
	rm -rf tests/
	umount fs/ ; rm -rf fs/
