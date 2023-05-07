clean:
	cd ./user && make clean
	cd ./tests_booter && rm -rf bin/

ifeq ($(shell id -u), 0)
  SU =
else
  SU = sudo
endif

tests:
	cd ./user && $(SU) ./build-oscomp.sh

initproc:
	cd tests_booter && make initproc
