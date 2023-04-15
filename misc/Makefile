clean:
	cd ./user && make clean
	cd ./tests_booter && rm -rf bin

tests:
	find ./user/build/riscv64 || (cd ./user && sudo bash ./build-oscomp.sh)

initproc:
	cd tests_booter && make initproc
