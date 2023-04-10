# Bite The Disk

## 项目结构
| 目录       | 简述                       |
| ---------- | -------------------------- |
| .vscode    | VSCode, RA, C/C++ 相关配置 |
| docs       | 项目相关文档               |
| fat32      | FAT32 文件系统             |
| misc       | 系统调用测例，util 脚本等  |
| os         | 内核源码，SBI             |
| workspace    | 用于做一下临时的挂载等任务，方便内核调试 |

```
# 区域赛完整的 qemu 命令
@qemu-system-riscv64 \
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
- 项目根目录下的在 dev 只会用到 `clean` 这个目标
- 有关 os 目录下的:
  ``` 
    kernel: 以 release 模式构建内核
    fat32img: 在本地构建一个 300 MiB 的 fat32 格式的镜像，并将 syscall tests 编译后打包进该镜像，以供本地 qemu 使用
    run: 在本地环境运行（一般用来在本地看测例的结果）
    kernel-debug: 以 debug 模式构建内核
    debug-server: 开启一个 debug server
    debug: 使用 gdb 连接 debug-server 开始调试
  ```
  debug 建议开启两个 shell，一个 make debug-server，另一个等前一个 shell 进入等待后 make debug
  debug 相关文档：[docs/debug.md](docs/debug.md)
