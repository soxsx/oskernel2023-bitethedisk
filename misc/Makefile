clean:
	cd ./user && make clean
	cd ./tests_booter && rm -rf bin/

ifeq ($(shell id -u), 0)
  SH = bash
else
  SH = sudo bash
endif

tests:
	# @find ./user/build/riscv64 || (cd ./user && $(SH) ./build-oscomp.sh)
	@cd ./user && $(SH) ./build-oscomp.sh

initproc:
	@cd tests_booter && make initproc