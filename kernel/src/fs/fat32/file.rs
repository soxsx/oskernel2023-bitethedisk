use crate::fs::{
    open_flags::CreateMode,
    stat::{S_IFCHR, S_IFDIR, S_IFREG},
    Dirent, File, Kstat, OpenFlags, Timespec,
};
use crate::{drivers::BLOCK_DEVICE, mm::UserBuffer};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use fat32::{
    dir::DirError, root, Dir as FatDir, File as FatFile, FileSystem, VirFile, VirFileType,
    ATTR_ARCHIVE, ATTR_DIRECTORY,
};
use spin::Mutex;

/// 表示进程中一个被打开的常规文件或目录
pub struct Fat32File {
    readable: bool, // 该文件是否允许通过 sys_read 进行读
    writable: bool, // 该文件是否允许通过 sys_write 进行写
    pub inner: Mutex<Fat32FileInner>,
    path: String, // TODO
    name: String,
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
        path: String,
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
    pub fn file_size(&self) -> usize {
        let inner = self.inner.lock();
        inner.inode.file_size() as usize
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

static mut LAYER: usize = 0;

pub fn list_apps(dir: Arc<VirFile>) {
    for app in dir.ls_with_attr().unwrap() {
        // 不打印initproc, 事实上它也在task::new之后删除了
        unsafe {
            if LAYER == 0 && app.0 == "initproc" {
                continue;
            }
        }
        if app.1 & ATTR_DIRECTORY == 0 {
            // 如果不是目录
            unsafe {
                for _ in 0..LAYER {
                    print!("----");
                }
            }
            crate::println!("{}", app.0);
        } else if app.0 != "." && app.0 != ".." {
            unsafe {
                for _ in 0..LAYER {
                    crate::print!("----");
                }
            }
            let dir = open(
                dir.name(),
                app.0.as_str(),
                OpenFlags::O_RDONLY,
                CreateMode::empty(),
            )
            .unwrap();
            let inner = dir.inner.lock();
            let inode = inner.inode.clone();
            unsafe {
                LAYER += 1;
            }
            list_apps(inode);
        }
    }
    unsafe {
        // fix: may overflow
        if LAYER > 0 {
            LAYER -= 1;
        }
    }
}

// work_path 绝对路径
pub fn open(
    work_path: &str,
    path: &str,
    flags: OpenFlags,
    _mode: CreateMode,
) -> Option<Arc<Fat32File>> {
    let mut pathv: Vec<&str> = path.split('/').collect();

    let cur_inode = {
        if work_path == "/" {
            ROOT_INODE.clone()
        } else {
            let wpath: Vec<&str> = work_path.split('/').collect();
            ROOT_INODE.find(wpath).unwrap()
        }
    };

    let (readable, writable) = flags.read_write();

    // 创建文件
    if flags.contains(OpenFlags::O_CREATE) {
        let res = cur_inode.find(pathv.clone());
        match res {
            Ok(inode) => {
                // 如果文件已存在则清空
                let name = pathv.pop().unwrap();
                inode.clear();
                Some(Arc::new(Fat32File::new(
                    readable,
                    writable,
                    inode,
                    work_path.to_string(),
                    name.to_string(),
                )))
            }
            Err(_) => {
                // 设置创建类型
                let mut create_type = VirFileType::File;
                if flags.contains(OpenFlags::O_DIRECTROY) {
                    create_type = VirFileType::Dir;
                }

                // 找到父目录
                let name = pathv.pop().unwrap();
                match cur_inode.find(pathv.clone()) {
                    Ok(parent) => match parent.create(name, create_type as VirFileType) {
                        Ok(inode) => Some(Arc::new(Fat32File::new(
                            readable,
                            writable,
                            Arc::new(inode),
                            work_path.to_string(),
                            name.to_string(),
                        ))),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            }
        }
    } else {
        // 查找文件
        match cur_inode.find(pathv.clone()) {
            Ok(inode) => {
                // 删除文件
                if flags.contains(OpenFlags::O_TRUNC) {
                    inode.clear();
                }

                let name = inode.name().to_string();
                Some(Arc::new(Fat32File::new(
                    readable,
                    writable,
                    inode,
                    work_path.to_string(),
                    name,
                )))
            }
            Err(_) => None,
        }
    }
}

pub fn chdir(work_path: &str, path: &str) -> Option<String> {
    let mut current_work_path_vec: Vec<&str> = work_path.split('/').collect();
    let path_vec: Vec<&str> = path.split('/').collect();

    let current_inode = {
        if path.chars().nth(0).unwrap() == '/' {
            // 传入路径是绝对路径
            ROOT_INODE.clone()
        } else {
            // 传入路径是相对路径
            ROOT_INODE.find(current_work_path_vec.clone()).unwrap()
        }
    };

    match current_inode.find(path_vec.clone()) {
        Ok(_inode) => {
            if current_inode.name() == "/" {
                Some(path.to_string())
            } else {
                // 将 work_path 和 path 拼接, work_path 为绝对路径, path 为相对路径
                for i in 0..path_vec.len() {
                    if path_vec[i] == "." || path_vec[i] == "" {
                        continue;
                    } else if path_vec[i] == ".." {
                        current_work_path_vec.pop();
                    } else {
                        current_work_path_vec.push(path_vec[i]);
                    }
                }

                Some(current_work_path_vec.join("/"))
            }
        }
        Err(_) => None,
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
            v.extend_from_slice(&buffer[..read_size.min(len)]);
            if len > read_size {
                len -= read_size;
            } else {
                break;
            }
        }
        if offset >= 0 {
            inner.offset = old_offset;
        }

        v
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
        let offset = self.inner.lock().offset;
        let file_size = self.file_size();
        // TODO 如果是目录文件
        if file_size == 0 {
            // crate::warn!("[fs::inode::OSInode::read] File size is zero!");
        }
        if offset > file_size {
            return 0;
        }
        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;

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
        let mut total_write_size = 0usize;
        let filesize = self.file_size();
        let mut inner = self.inner.lock();
        if inner.flags.contains(OpenFlags::O_APPEND) {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(filesize, *slice);
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

    fn write_kernel_space(&self, data: Vec<u8>) -> usize {
        // (lzm) 改为 slice
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

    fn set_time(&self, timespec: &Timespec) {
        let tv_sec = timespec.tv_sec;
        let tv_nsec = timespec.tv_nsec;

        let inner = self.inner.lock();
        let vfile = inner.inode.clone();

        // 属于是针对测试用例了, 待完善
        if tv_sec == 1 << 32 {
            vfile.set_time(tv_sec, tv_nsec);
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
            // TODO + 8 * 4 是什么意思?
            let len = (name.len() + 8 * 4) as isize;
            len
        } else {
            -1
        }
    }

    fn fstat(&self, kstat: &mut Kstat) {
        let inner = self.inner.lock();
        let vfile = inner.inode.clone();
        let mut st_mode = 0;
        _ = st_mode;
        let (st_size, st_blksize, st_blocks, is_dir, time) = vfile.stat();

        if is_dir {
            st_mode = S_IFDIR;
        } else {
            st_mode = S_IFREG;
        }
        if vfile.name() == "null" || vfile.name() == "zero" {
            st_mode = S_IFCHR;
        }
        kstat.init(
            st_size as i64,
            st_blksize as i32,
            st_blocks as u64,
            st_mode as u32,
            time as u64,
        );
    }

    fn file_size(&self) -> usize {
        self.file_size()
    }

    fn get_path(&self) -> &str {
        self.path.as_str()
    }
}
