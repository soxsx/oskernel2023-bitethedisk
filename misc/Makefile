clean:
	@cd ./user && make clean

ifeq ($(shell id -u), 0)
  SH = bash
else
  SH = sudo bash
endif

tests:
	@find ./user/build/riscv64 || (cd ./user && $(SH) ./build-oscomp.sh)
