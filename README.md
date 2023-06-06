# Bite The Disk

![Rust](https://img.shields.io/badge/programming--lang-Rust-red?style=for-the-badge&logo=rust)
![Platform](https://img.shields.io/badge/platform-qemu-blue?style=for-the-badge&logo=qemu)
![TODO](https://img.shields.io/badge/platform--todo-Hifive--Unmatched-yellow?style=for-the-badge&logo=Hifive-Unmatched)


## Intro

该项目由 HDU 三位同学在历届内核以及 Linux 实现的基础上，参考实现的一个可支持多核 CPU 的简单 OS 内核

## Features

### IO

- 完全支持标准 FAT32 镜像的读写，LRUCache

### 进程

- signal
- ...

### 内存

- lazy
- COW
- ...

## 项目概况

### 项目结构
| 目录      | 简述                                     |
| --------- | ---------------------------------------- |
| .vscode   | VSCode, RA, C/C++ 相关配置               |
| docs      | 项目相关文档                             |
| fat32     | FAT32 文件系统                           |
| misc      | 系统调用测例，util 脚本等                |
| kernel    | 内核源码，SBI                            |
| workspace | 用于做一下临时的挂载等任务，方便内核调试 |

### Build

```shell
make run
make debug-server
make debug
...
```
### Next Step

利用 `Hifive-Unmatched` 大小核特性，参考 goroutine 尝试优化进程调度

## 遇到的问题与解决

- 工具链默认不支持乘法指令
- VirtIOBlk 物理内存不联系时导致缓存数据丢失
- 多核乱序输出
- ...

## 关联文档 / 链接 / 相关 `issue`

- https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md#-attribute
- debug.md
- commit-spec.md