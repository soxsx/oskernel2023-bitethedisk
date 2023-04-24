// 定义一份打开文件的标志
bitflags! {
    #[derive(Debug)]
    pub struct OpenFlags: u32 {
        const O_RDONLY    = 0;
        const O_WRONLY    = 1 << 0;
        const O_RDWR      = 1 << 1;
        const O_CREATE    = 1 << 6;
        const O_EXCL      = 1 << 7;
        const O_TRUNC     = 1 << 9;
        const O_APPEND    = 1 << 10;
        const O_NONBLOCK  = 1 << 11;
        const O_LARGEFILE = 1 << 15;
        const O_DIRECTROY = 1 << 16;
        const O_NOFOLLOW  = 1 << 17;
        const O_CLOEXEC   = 1 << 19;
    }

    /// 用户组读写权限
    #[derive(Debug)]
    pub struct CreateMode: u32 {
        const S_ISUID  = 0o4000;
        const S_ISGID  = 0o2000;
        const S_ISVTX  = 0o1000;
        
        const S_IRWXU  = 0o700;
        const S_IRUSR  = 0o400;
        const S_IWUSR  = 0o200;
        const S_IXUSR  = 0o100;
        
        const S_IRWXG  = 0o070;
        const S_IRGRP  = 0o040;
        const S_IWGRP  = 0o020;
        const S_IXGRP  = 0o010;
        
        const S_IRWXO  = 0o007;
        const S_IROTH  = 0o004;
        const S_IWOTH  = 0o002;
        const S_IXOTH  = 0o001;
    }
}

impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::O_WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}
//    S_IFMT     0170000   bit mask for the file type bit field
//    S_IFSOCK   0140000   socket
//    S_IFLNK    0120000   symbolic link
//    S_IFREG    0100000   regular file
//    S_IFBLK    0060000   block device
//    S_IFDIR    0040000   directory
//    S_IFCHR    0020000   character device
//    S_IFIFO    0010000   FIFO

//    S_ISREG(m)  is it a regular file?
//    S_ISDIR(m)  directory?
//    S_ISCHR(m)  character device?
//    S_ISBLK(m)  block device?
//    S_ISFIFO(m) FIFO (named pipe)?
//    S_ISLNK(m)  symbolic link?  (Not in POSIX.1-1996.)
//    S_ISSOCK(m) socket?  (Not in POSIX.1-1996.)

//    S_ISUID     04000   set-user-ID bit (see execve(2))
//    S_ISGID     02000   set-group-ID bit (see below)
//    S_ISVTX     01000   sticky bit (see below)

//    S_IRWXU     00700   owner has read, write, and execute permission
//    S_IRUSR     00400   owner has read permission
//    S_IWUSR     00200   owner has write permission
//    S_IXUSR     00100   owner has execute permission

//    S_IRWXG     00070   group has read, write, and execute permission
//    S_IRGRP     00040   group has read permission
//    S_IWGRP     00020   group has write permission
//    S_IXGRP     00010   group has execute permission

//    S_IRWXO     00007   others (not in group) have read,  write,  and
//                        execute permission
//    S_IROTH     00004   others have read permission
//    S_IWOTH     00002   others have write permission
//    S_IXOTH     00001   others have execute permission
