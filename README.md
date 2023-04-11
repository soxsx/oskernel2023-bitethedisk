# Bite The Disk

## 项目结构
| 目录      | 简述                                     |
| --------- | ---------------------------------------- |
| .vscode   | VSCode, RA, C/C++ 相关配置               |
| docs      | 项目相关文档                             |
| fat32     | FAT32 文件系统                           |
| misc      | 系统调用测例，util 脚本等                |
| os        | 内核源码，SBI                            |
| workspace | 用于做一下临时的挂载等任务，方便内核调试 |

```
# 区域赛完整的 qemu 命令
qemu-system-riscv64 \
    -machine virt \
    -kernel kernel-qemu \
    -m 128M \
    -nographic \
    -smp 2 \
    -bios sbi-qemu \
    -drive file=sdcard.img,if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -initrd initrd.img
```

## 有关 Makefile
一般只会用到项目根目录中的 [Makefile](./Makefile)

```shell
sbi-qemu:
    将 ./os/bootloader/ 中的 SBI 改名复制到根目录

kernel-qemu:
    按照 release 编译内核后将其改名放到根目录

all: sbi-qemu kernel-qemu

clean:
	rm -f kernel-qemu
	rm -f sbi-qemu
	rm -rf build/
	rm -rf temp/
	cd os/ && cargo clean
	cd workspace/ && make clean
	cd fat32/ && cargo clean
	cd misc/user && make clean

fat32img: 
    构建一个 fat32 格式 300 MiB 大小的镜像，并将编译后的测例拷贝进去（测例的编译结果会复用，
    除非删除 ./misc/user/riscv64 这个文件夹，可使用上面的 `clean` 一键清空所以先前的构建
    还原到项目最干净的状态）

run:
    将内核按 release 编译后，使用本地构建的 fat32 镜像（见上面 fat32img），挂载到 qemu 运行

debug-server:
    将内核按照 debug 编译，搭载 fat32 （见上面 fat32img），挂在 qemu 启动 debug server

debug:
    连接上面 `debug-server` 启动的服务，用 gdb 开始调试

```

debug 建议开启两个 shell，一个 make debug-server，另一个等前一个 shell 进入等待后 make debug

debug 相关文档：[docs/debug.md](docs/debug.md)
