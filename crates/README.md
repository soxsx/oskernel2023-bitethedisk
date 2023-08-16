# crates

这里放着的是 BTD-OS 依赖的库

- fat32

  解析读写 FAT32 镜像

- libd

  适用于 BTD-OS 的用户库(~~无端联想 libc~~)

- sync_cell

  应对 `rustc` 检查的全局 Cell

- nix

  我们以库的方式将 POSIX 要求的数据结构分离出来，这些数据结构可能并非内核必须的。这样做以达到简化内核结构的目的。