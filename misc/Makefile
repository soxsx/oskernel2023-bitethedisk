clean:
	cd ./user && make clean
	cd ./test_booter && rm -rf bin

tests:
	find ./user/build/riscv64 || (cd ./user && sudo bash ./build-oscomp.sh)

initproc:
	cd test_booter && make initproc
