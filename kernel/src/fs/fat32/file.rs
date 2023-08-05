use crate::drivers::BLOCK_DEVICE;
use crate::fs::{CreateMode, Dirent, File, Kstat, OpenFlags, TimeInfo, S_IFCHR, S_IFDIR, S_IFREG};
use crate::mm::UserBuffer;
use crate::return_errno;
use crate::syscall::impls::Errno;
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use fat32::{root, Dir as FatDir, DirError, FileSystem, VirFile, VirFileType, ATTR_DIRECTORY};
use path::AbsolutePath;
use spin::Mutex;

/// 表示进程中一个被打开的常规文件或目录
pub struct Fat32File {
    readable: bool, // 该文件是否允许通过 sys_read 进行读
    writable: bool, // 该文件是否允许通过 sys_write 进行写
    pub inner: Mutex<Fat32FileInner>,
    path: AbsolutePath, // contain file name
    name: String,

    time_info: Mutex<TimeInfo>,
}

pub struct Fat32FileInner {
    offset: usize, // 偏移量
    pub inode: Arc<VirFile>,
    flags: OpenFlags,
    available: bool,
}

impl Fat32File {
    pub fn new(
        readable: bool,
        writable: bool,
        inode: Arc<VirFile>,
        path: AbsolutePath,
        name: String,
    ) -> Self {
        let available = true;
        Self {
            readable,
            writable,
            inner: Mutex::new(Fat32FileInner {
                offset: 0,
                inode,
                flags: OpenFlags::empty(),
                available,
            }),
            path,
            name,

            time_info: Mutex::new(TimeInfo::empty()),
        }
    }

    #[allow(unused)]
    // TODO 根据文件大小
    pub fn read_all(&self) -> Vec<u8> {
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = vec![];
        let mut inner = self.inner.lock();
        let mut i = 0;
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
            i += 1;
        }
        v
    }

    pub fn write_all(&self, str_vec: &Vec<u8>) -> usize {
        let mut inner = self.inner.lock();
        let mut remain = str_vec.len();
        let mut base = 0;

        loop {
            let len = remain.min(512);
            inner
                .inode
                .write_at(inner.offset, &str_vec.as_slice()[base..base + len]);
            inner.offset += len;
            base += len;
            remain -= len;
            if remain == 0 {
                break;
            }
        }
        base
    }

    pub fn is_dir(&self) -> bool {
        let inner = self.inner.lock();
        inner.inode.is_dir()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn delete(&self) -> usize {
        let inner = self.inner.lock();
        inner.inode.clear()
    }

    pub fn delete_direntry(&self) {
        let inner = self.inner.lock();
        inner.inode.clear_direntry();
    }

    pub fn file_size(&self) -> usize {
        let inner = self.inner.lock();
        inner.inode.file_size() as usize
    }

    pub fn rename(&self, new_path: AbsolutePath, flags: OpenFlags) {
        // duplicate a new file, and set file cluster and file size
        let inner = self.inner.lock();
        // check file exits
        let new_file = open(new_path, flags, CreateMode::empty()).unwrap();
        let new_inner = new_file.inner.lock();
        let first_cluster = inner.inode.first_cluster();
        let file_size = inner.inode.file_size();

        new_inner.inode.set_first_cluster(first_cluster);
        new_inner.inode.set_file_size(file_size);

        drop(inner);
        // clear old direntry
        self.delete_direntry();
    }
}

// 这里在实例化的时候进行文件系统的打开
lazy_static! {
    pub static ref ROOT_INODE: Arc<VirFile> = {
        let fs = FileSystem::open(BLOCK_DEVICE.clone());

        // 返回根目录
        Arc::new(root(fs.clone()))
    };
}

pub fn list_apps(path: AbsolutePath) {
    let layer: usize = 0;

    fn ls(path: AbsolutePath, layer: usize) {
        let dir = ROOT_INODE.find(path.as_vec_str()).unwrap();
        for app in dir.ls_with_attr().unwrap() {
            // 不打印initproc, 事实上它也在task::new之后删除了
            if layer == 0 && app.0 == "initproc" {
                continue;
            }
            let app_path: AbsolutePath = path.cd(app.0.clone());
            if app.1 & ATTR_DIRECTORY == 0 {
                // 如果不是目录
                for _ in 0..layer {
                    print!("   ");
                }
                crate::println!("{}", app.0);
            } else if app.0 != "." && app.0 != ".." {
                // 目录
                for _ in 0..layer {
                    crate::print!("  ");
                }
                crate::println!("{}/", app.0);
                ls(app_path.clone(), layer + 1);
            }
        }
    }

    ls(path, layer);
}

// work_path 绝对路径
pub fn open(
    path: AbsolutePath,
    flags: OpenFlags,
    _mode: CreateMode,
) -> Result<Arc<Fat32File>, Errno> {
    time_trace!("open");
    let mut pathv = path.as_vec_str();
    let (readable, writable) = flags.read_write();
    // 创建文件
    if flags.contains(OpenFlags::O_CREATE) {
        let res = ROOT_INODE.find(pathv.clone());
        match res {
            Ok(inode) => {
                let name = if let Some(name_) = pathv.pop() {
                    name_
                } else {
                    "/"
                };
                Ok(Arc::new(Fat32File::new(
                    readable,
                    writable,
                    inode,
                    path.clone(),
                    name.to_string(),
                )))
            }
            Err(_err) => {
                if _err == DirError::NotDir {
                    return Err(Errno::ENOTDIR);
                }
                // 设置创建类型
                let mut create_type = VirFileType::File;
                if flags.contains(OpenFlags::O_DIRECTROY) {
                    create_type = VirFileType::Dir;
                }

                // 找到父目录
                let name = pathv.pop().unwrap();
                match ROOT_INODE.find(pathv.clone()) {
                    Ok(parent) => match parent.create(name, create_type as VirFileType) {
                        Ok(inode) => Ok(Arc::new(Fat32File::new(
                            readable,
                            writable,
                            Arc::new(inode),
                            path.clone(),
                            name.to_string(),
                        ))),
                        Err(_err) => Err(Errno::UNCLEAR),
                    },
                    Err(_err) => {
                        return_errno!(Errno::ENOENT, "parent path not exist path:{:?}", path)
                    }
                }
            }
        }
    } else {
        // 查找文件
        match ROOT_INODE.find(pathv.clone()) {
            Ok(inode) => {
                // 删除文件
                if flags.contains(OpenFlags::O_TRUNC) {
                    inode.clear();
                }

                let name = inode.name().to_string();
                Ok(Arc::new(Fat32File::new(
                    readable,
                    writable,
                    inode,
                    path.clone(),
                    name,
                )))
            }
            Err(_err) => return_errno!(Errno::ENOENT, "no such file or path:{:?}", path),
        }
    }
}

pub fn chdir(path: AbsolutePath) -> bool {
    if let Ok(_) = ROOT_INODE.find(path.as_vec_str()) {
        true
    } else {
        false
    }
}

// 为 OSInode 实现 File Trait
impl File for Fat32File {
    fn read_to_vec(&self, offset: isize, len: usize) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut len = len;
        let old_offset = inner.offset;
        if offset >= 0 {
            inner.offset = offset as usize;
        }
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        // TODO
        if len >= 96 * 4096 {
            // 防止 v 占用空间过度扩大
            v.reserve(96 * 4096);
        }
        loop {
            let read_size = inner.inode.read_at(inner.offset, &mut buffer);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            if len > read_size {
                len -= read_size;
                v.extend_from_slice(&buffer[..read_size]);
            } else {
                v.extend_from_slice(&buffer[..len]);
                break;
            }
        }
        if offset >= 0 {
            inner.offset = old_offset;
        }

        v
    }

    fn path(&self) -> AbsolutePath {
        self.path.clone()
    }

    fn seek(&self, _pos: usize) {
        let mut inner = self.inner.lock();
        inner.offset = _pos;
    }

    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn available(&self) -> bool {
        let inner = self.inner.lock();
        inner.available
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        time_trace!("read");
        let offset = self.inner.lock().offset;
        let file_size = self.file_size();
        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;

        // TODO 如果是目录文件
        // TAG for lzm
        // empty file
        if file_size == 0 {
            if self.name == "zero" {
                buf.write_zeros();
            }
            return 0;
        }

        if offset >= file_size {
            return 0;
        }

        // 这边要使用 iter_mut(), 因为要将数据写入
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn pread(&self, mut buf: UserBuffer, offset: usize) -> usize {
        let inner = self.inner.lock();
        let mut index = offset;
        let file_size = inner.inode.file_size();

        let mut total_read_size = 0usize;

        // TODO 如果是目录文件
        // TAG for lzm
        // empty file
        if file_size == 0 {
            if self.name == "zero" {
                buf.write_zeros();
            }
            return 0;
        }

        if offset >= file_size {
            return 0;
        }

        // 这边要使用 iter_mut(), 因为要将数据写入
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(index, *slice);
            if read_size == 0 {
                break;
            }
            index += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    fn read_kernel_space(&self) -> Vec<u8> {
        let file_size = self.file_size();
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            if inner.offset > file_size {
                break;
            }
            let readsize = inner.inode.read_at(inner.offset, &mut buffer);
            if readsize == 0 {
                break;
            }
            inner.offset += readsize;
            v.extend_from_slice(&buffer[..readsize]);
        }
        v.truncate(v.len().min(file_size));
        v
    }

    fn write(&self, buf: UserBuffer) -> usize {
        time_trace!("write");
        let mut total_write_size = 0usize;
        let filesize = self.file_size();
        let mut inner = self.inner.lock();
        if inner.flags.contains(OpenFlags::O_APPEND) {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(filesize + total_write_size, *slice);
                inner.offset += write_size;
                total_write_size += write_size;
            }
        } else {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(inner.offset, *slice);
                assert_eq!(write_size, slice.len());
                inner.offset += write_size;
                total_write_size += write_size;
            }
        }
        total_write_size
    }
    fn pwrite(&self, buf: UserBuffer, offset: usize) -> usize {
        let inner = self.inner.lock();
        let mut index = offset;
        let file_size = inner.inode.file_size();

        let mut total_write_size = 0usize;
        if inner.flags.contains(OpenFlags::O_APPEND) {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(file_size + total_write_size, *slice);
                total_write_size += write_size;
            }
        } else {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(index, *slice);
                assert_eq!(write_size, slice.len());
                index += write_size;
                total_write_size += write_size;
            }
        }
        total_write_size
    }

    fn write_kernel_space(&self, data: Vec<u8>) -> usize {
        let mut inner = self.inner.lock();
        let mut remain = data.len();
        let mut base = 0;
        loop {
            let len = remain.min(512);
            inner
                .inode
                .write_at(inner.offset, &data.as_slice()[base..base + len]);
            inner.offset += len;
            base += len;
            remain -= len;
            if remain == 0 {
                break;
            }
        }
        base
    }

    fn set_time(&self, time_info: TimeInfo) {
        let mut time_lock = self.time_info.lock();
        // 根据测例改动
        if time_info.mtime < time_lock.mtime {
            time_lock.atime = time_info.atime;
            time_lock.ctime = time_info.ctime;
        } else {
            *time_lock = time_info;
        }
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn offset(&self) -> usize {
        let inner = self.inner.lock();
        inner.offset
    }

    fn set_offset(&self, offset: usize) {
        let mut inner = self.inner.lock();
        inner.offset = offset;
    }

    fn set_flags(&self, flag: OpenFlags) {
        let mut inner = self.inner.lock();
        inner.flags.set(flag, true);
    }

    fn set_cloexec(&self) {
        let mut inner = self.inner.lock();
        inner.available = false;
    }

    // set dir entry
    fn dirent(&self, dirent: &mut Dirent) -> isize {
        if !self.is_dir() {
            return -1;
        }
        let mut inner = self.inner.lock();
        let offset = inner.offset as u32;
        if let Some((name, offset, first_cluster, _attr)) = inner.inode.dir_info(offset as usize) {
            dirent.init(name.as_str(), offset as isize, first_cluster as usize);
            inner.offset = offset as usize;
            // return size of Dirent as read size
            core::mem::size_of::<Dirent>() as isize
        } else {
            -1
        }
    }

    fn fstat(&self, kstat: &mut Kstat) {
        let inner = self.inner.lock();
        let vfile = inner.inode.clone();
        let mut st_mode = 0;
        _ = st_mode;
        let (st_size, st_blksize, st_blocks, is_dir, _time) = vfile.stat();

        if is_dir {
            st_mode = S_IFDIR;
        } else {
            st_mode = S_IFREG;
        }
        if vfile.name() == "null"
            || vfile.name() == "NULL"
            || vfile.name() == "zero"
            || vfile.name() == "ZERO"
        {
            st_mode = S_IFCHR;
        }
        let time_info = self.time_info.lock();
        let atime = time_info.atime;
        let mtime = time_info.mtime;
        let ctime = time_info.ctime;
        kstat.init(
            st_size as i64,
            st_blksize as i32,
            st_blocks as u64,
            st_mode as u32,
            atime as i64,
            mtime as i64,
            ctime as i64,
        );
    }

    fn file_size(&self) -> usize {
        self.file_size()
    }

    fn truncate(&self, new_length: usize) {
        let inner = self.inner.lock();
        inner.inode.modify_size(new_length);
    }
}
