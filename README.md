# Bite The Disk

## 项目结构
| 目录       | 简述                       |
| ---------- | -------------------------- |
| .vscode    | VSCode, RA, C/C++ 相关配置 |
| bootloader | RustSBI(bin)               |
| docs       | 项目相关文档               |
| fat32      | FAT32 文件系统             |
| misc       | 系统调用测例，util 脚本等  |
| os         | 内核源码                   |
| vendor     | 可能用到的特定依赖         |
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