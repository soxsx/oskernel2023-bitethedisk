use crate::consts::PAGE_SIZE;
use crate::drivers::BLOCK_DEVICE;
use crate::fs::{
    CreateMode, Dirent, File, Kstat, OpenFlags, PageCache, TimeInfo, S_IFCHR, S_IFDIR, S_IFREG,
};
use crate::mm::UserBuffer;
use crate::return_errno;
use crate::syscall::impls::Errno;
use alloc::collections::BTreeMap;
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use fat32::{root, Dir as FatDir, DirError, FileSystem, VirtFile, VirtFileType, ATTR_DIRECTORY};
use path::AbsolutePath;
use spin::{Mutex, MutexGuard, RwLock};

pub struct InodeCache(pub RwLock<BTreeMap<AbsolutePath, Arc<Inode>>>);

pub static INODE_CACHE: InodeCache = InodeCache(RwLock::new(BTreeMap::new()));

impl InodeCache {
    pub fn get(&self, path: &AbsolutePath) -> Option<Arc<Inode>> {
        self.0.read().get(path).cloned()
    }

    pub fn insert(&self, path: AbsolutePath, inode: Arc<Inode>) {
        self.0.write().insert(path, inode);
    }

    pub fn remove(&self, path: &AbsolutePath) {
        self.0.write().remove(path);
    }
}

/// 表示进程中一个被打开的常规文件或目录
pub struct Fat32File {
    readable: bool,     // 该文件是否允许通过 sys_read 进行读
    writable: bool,     // 该文件是否允许通过 sys_write 进行写
    path: AbsolutePath, // contain file name
    name: String,
    pub inode: Arc<Inode>,
    // pub page_cache: Mutex<Option<Arc<PageCache>>>,
    pub time_info: Mutex<TimeInfo>,
    pub offset: Mutex<usize>,
    // pub inner: Mutex<Arc<VirtFile>>,
    pub flags: Mutex<OpenFlags>,
    pub available: Mutex<bool>,
}

pub struct Inode {
    pub file: Mutex<Arc<VirtFile>>,
    pub page_cache: Mutex<Option<Arc<PageCache>>>,
    pub file_size: Mutex<usize>,
}

impl Fat32File {
    pub fn new(
        readable: bool,
        writable: bool,
        inode: Arc<Inode>,
        path: AbsolutePath,
        name: String,
    ) -> Self {
        let available = true;
        let file_size = inode.file.lock().file_size() as usize;
        Self {
            readable,
            writable,
            // inner: Mutex::new(inode),
            path,
            name,
            // page_cache: Mutex::new(None),
            inode,
            offset: Mutex::new(0),
            flags: Mutex::new(OpenFlags::empty()),
            available: Mutex::new(available),
            time_info: Mutex::new(TimeInfo::empty()),
        }
    }

    pub fn inner(&self) -> MutexGuard<'_, Arc<VirtFile>> {
        self.inode.file.lock()
    }

    pub fn page_cache(&self) -> MutexGuard<'_, Option<Arc<PageCache>>> {
        self.inode.page_cache.lock()
    }

    pub fn create_page_cache_if_needed(self: &Arc<Self>) {
        let mut page_cache = self.page_cache();
        if page_cache.is_none() {
            // *page_cache = Some(Arc::new(PageCache::new(self.clone())));
            *page_cache = Some(Arc::new(PageCache::new(self.inner().clone())));
        }
    }

    pub fn write_all(&self, data: &Vec<u8>) -> usize {
        // with page cache
        let mut total_write_size = 0usize;
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        let mut offset = if self.flags().contains(OpenFlags::O_APPEND) {
            self.file_size()
        } else {
            self.offset()
        };
        let mut slice_offset = 0;
        let slice_end = data.len();
        while slice_offset < slice_end {
            // to avoid slice's length spread page boundary (howerver, it's low probability)
            let page = page_cache.get_page(offset, None).expect("get page error");
            let page_offset = offset % PAGE_SIZE;
            let mut slice_offset_end = slice_offset + (PAGE_SIZE - page_offset);
            if slice_offset_end > slice_end {
                slice_offset_end = slice_end;
            }
            let write_size = page
                .write(page_offset, &data[slice_offset..slice_offset_end])
                .expect("read page error");
            offset += write_size;
            self.seek(offset);
            slice_offset += write_size;
            total_write_size += write_size;
        }
        if self.file_size() < offset {
            self.set_file_size(offset);
        }
        total_write_size
    }
    pub fn is_dir(&self) -> bool {
        let inner = self.inner();
        inner.is_dir()
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    // TODO with page cache ? lzm
    pub fn delete(&self) -> usize {
        let inner = self.inner();
        inner.clear()
    }
    // TODO with page cache ? lzm
    pub fn delete_direntry(&self) {
        let inner = self.inner();
        inner.clear_direntry();
    }
    // TODO with page cache ? lzm
    pub fn file_size(&self) -> usize {
        // *self.file_size.lock()
        *self.inode.file_size.lock()
    }
    // TODO with page cache ? lzm
    pub fn set_file_size(&self, file_size: usize) {
        // *self.file_size.lock() = file_size;
        *self.inode.file_size.lock() = file_size;
    }
    pub fn rename(&self, new_path: AbsolutePath, flags: OpenFlags) {
        // duplicate a new file, and set file cluster and file size
        let inner = self.inner();
        // check file exits
        let new_file = open(new_path, flags, CreateMode::empty()).unwrap();
        let new_inner = new_file.inner();
        let first_cluster = inner.first_cluster();
        let file_size = inner.file_size();

        new_inner.set_first_cluster(first_cluster);
        new_inner.set_file_size(file_size);

        drop(inner);
        // clear old direntry
        self.delete_direntry();
    }
}

// 这里在实例化的时候进行文件系统的打开
lazy_static! {
    pub static ref ROOT_INODE: Arc<VirtFile> = {
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
    let (readable, writable) = flags.read_write();
    let mut pathv = path.as_vec_str();
    if let Some(inode) = INODE_CACHE.get(&path) {
        let name = if let Some(name_) = pathv.last() {
            name_.to_string()
        } else {
            "/".to_string()
        };
        let res = Arc::new(Fat32File::new(
            readable,
            writable,
            inode.clone(),
            path.clone(),
            name,
        ));
        res.create_page_cache_if_needed();
        return Ok(res);
    }
    // 创建文件
    if flags.contains(OpenFlags::O_CREATE) {
        let res = ROOT_INODE.find(pathv.clone());
        match res {
            Ok(file) => {
                let name = if let Some(name_) = pathv.pop() {
                    name_
                } else {
                    "/"
                };
                let file_size = file.file_size();
                let inode = Arc::new(Inode {
                    file: Mutex::new(file),
                    page_cache: Mutex::new(None),
                    file_size: Mutex::new(file_size),
                });

                let res = Arc::new(Fat32File::new(
                    readable,
                    writable,
                    inode.clone(),
                    path.clone(),
                    name.to_string(),
                ));
                res.create_page_cache_if_needed();
                INODE_CACHE.insert(path.clone(), inode.clone());
                Ok(res)
            }
            Err(_err) => {
                if _err == DirError::NotDir {
                    return Err(Errno::ENOTDIR);
                }
                // 设置创建类型
                let mut create_type = VirtFileType::File;
                if flags.contains(OpenFlags::O_DIRECTROY) {
                    create_type = VirtFileType::Dir;
                }

                // 找到父目录
                let name = pathv.pop().unwrap();
                match ROOT_INODE.find(pathv.clone()) {
                    Ok(parent) => match parent.create(name, create_type as VirtFileType) {
                        Ok(file) => {
                            let file_size = file.file_size();
                            let inode = Arc::new(Inode {
                                file: Mutex::new(Arc::new(file)),
                                page_cache: Mutex::new(None),
                                file_size: Mutex::new(file_size),
                            });
                            let res = Arc::new(Fat32File::new(
                                readable,
                                writable,
                                inode.clone(),
                                path.clone(),
                                name.to_string(),
                            ));
                            res.create_page_cache_if_needed();
                            INODE_CACHE.insert(path.clone(), inode.clone());
                            Ok(res)
                        }
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
            Ok(file) => {
                // 删除文件
                if flags.contains(OpenFlags::O_TRUNC) {
                    file.clear();
                }
                let name = file.name().to_string();
                let file_size = file.file_size();
                let inode = Arc::new(Inode {
                    file: Mutex::new(file),
                    file_size: Mutex::new(file_size),
                    page_cache: Mutex::new(None),
                });
                let res = Arc::new(Fat32File::new(
                    readable,
                    writable,
                    inode.clone(),
                    path.clone(),
                    name,
                ));
                res.create_page_cache_if_needed();
                INODE_CACHE.insert(path.clone(), inode.clone());
                Ok(res)
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
    //  No change file offset
    fn read_to_kspace_with_offset(&self, offset: usize, len: usize) -> Vec<u8> {
        // with page cache
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        let mut offset = offset;
        let mut buf: Vec<u8> = vec![0; len];
        let mut buf_offset = 0;
        let buf_end = len;

        while buf_offset < buf_end {
            // TODO error handle?
            let page = page_cache.get_page(offset, None).expect("get page error");
            let page_offset = offset % PAGE_SIZE;
            let mut buf_offset_end = buf_offset + (PAGE_SIZE - page_offset);
            if buf_offset_end > buf_end {
                buf_offset_end = buf_end;
            }
            let slice = buf.as_mut_slice();
            let read_size = page
                .read(page_offset, &mut slice[buf_offset..buf_offset_end])
                .expect("read page error");
            offset += read_size;
            buf_offset += read_size;
        }

        buf
    }

    fn read_at_direct(&self, offset: usize, len: usize) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![0; len];
        let inner = self.inner();
        inner.read_at(offset, &mut buf);
        buf
    }

    // change file offset
    // TODO lzm
    fn read_to_kspace(&self) -> Vec<u8> {
        let file_size = self.file_size();
        let offset = self.offset();
        let len = file_size - offset;
        let res = self.read_to_kspace_with_offset(offset, len);
        self.seek(offset + res.len());
        res
    }

    fn write_from_direct(&self, offset: usize, data: &Vec<u8>) -> usize {
        let inner = self.inner();
        // TODO lzm
        if offset + data.len() > self.file_size() {
            self.set_file_size(offset + data.len());
        }
        inner.write_at(offset, data)
    }

    fn path(&self) -> AbsolutePath {
        self.path.clone()
    }

    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn available(&self) -> bool {
        *self.available.lock()
    }

    fn read_to_ubuf(&self, mut buf: UserBuffer) -> usize {
        // with page cache
        time_trace!("read");
        let offset = self.offset();
        let file_size = self.file_size();
        let mut total_read_size = 0usize;
        if file_size == 0 {
            if self.name == "zero" {
                buf.write_zeros();
            }
            return 0;
        }
        if offset >= file_size {
            return 0;
        }
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        for slice in buf.buffers.iter_mut() {
            let slice_end = slice.len();
            let mut slice_offset = 0;
            while slice_offset < slice_end {
                // to avoid slice's length spread page boundary (howerver, it's low probability)
                let offset = self.offset();
                let page = page_cache.get_page(offset, None).expect("get page error");
                let page_offset = offset % PAGE_SIZE;
                let mut slice_offset_end = slice_offset + (PAGE_SIZE - page_offset);
                if slice_offset_end > slice_end {
                    slice_offset_end = slice_end;
                }
                let read_size = page
                    .read(page_offset, &mut slice[slice_offset..slice_offset_end])
                    .expect("read page error");
                self.seek(offset + read_size);
                slice_offset += read_size;
                total_read_size += read_size;
            }
        }
        total_read_size
    }
    // 同read_to_ubuf，只是不会改变offset
    fn pread(&self, mut buf: UserBuffer, offset: usize) -> usize {
        time_trace!("read");
        let mut offset = offset;
        let file_size = self.file_size();
        let mut total_read_size = 0usize;
        if file_size == 0 {
            if self.name == "zero" {
                buf.write_zeros();
            }
            return 0;
        }
        if offset >= file_size {
            return 0;
        }
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        for slice in buf.buffers.iter_mut() {
            let slice_end = slice.len();
            let mut slice_offset = 0;
            while slice_offset < slice_end {
                // to avoid slice's length spread page boundary (howerver, it's low probability)
                let page = page_cache.get_page(offset, None).expect("get page error");
                let page_offset = offset % PAGE_SIZE;
                let mut slice_offset_end = slice_offset + (PAGE_SIZE - page_offset);
                if slice_offset_end > slice_end {
                    slice_offset_end = slice_end;
                }
                let read_size = page
                    .read(page_offset, &mut slice[slice_offset..slice_offset_end])
                    .expect("read page error");
                offset += read_size;
                slice_offset += read_size;
                total_read_size += read_size;
            }
        }
        total_read_size
    }
    fn write_from_ubuf(&self, buf: UserBuffer) -> usize {
        time_trace!("write");
        let mut total_write_size = 0usize;
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        let mut offset = if self.flags().contains(OpenFlags::O_APPEND) {
            self.file_size()
        } else {
            self.offset()
        };
        for slice in buf.buffers.iter() {
            let slice_end = slice.len();
            let mut slice_offset = 0;
            while slice_offset < slice_end {
                // to avoid slice's length spread page boundary (howerver, it's low probability)
                let page = page_cache.get_page(offset, None).expect("get page error");
                let page_offset = offset % PAGE_SIZE;
                let mut slice_offset_end = slice_offset + (PAGE_SIZE - page_offset);
                if slice_offset_end > slice_end {
                    slice_offset_end = slice_end;
                }
                let write_size = page
                    .write(page_offset, &slice[slice_offset..slice_offset_end])
                    .expect("read page error");
                offset += write_size;
                self.seek(offset);
                slice_offset += write_size;
                total_write_size += write_size;
                // page.sync().expect("sync page error");
            }
        }
        if self.file_size() < offset {
            self.set_file_size(offset);
        }
        total_write_size
    }
    fn pwrite(&self, buf: UserBuffer, offset: usize) -> usize {
        time_trace!("write");
        let mut total_write_size = 0usize;
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        let mut offset = if self.flags().contains(OpenFlags::O_APPEND) {
            self.file_size()
        } else {
            offset
        };
        for slice in buf.buffers.iter() {
            let slice_end = slice.len();
            let mut slice_offset = 0;
            while slice_offset < slice_end {
                // to avoid slice's length spread page boundary (howerver, it's low probability)
                let page = page_cache.get_page(offset, None).expect("get page error");
                let page_offset = offset % PAGE_SIZE;
                let mut slice_offset_end = slice_offset + (PAGE_SIZE - page_offset);
                if slice_offset_end > slice_end {
                    slice_offset_end = slice_end;
                }
                let write_size = page
                    .write(page_offset, &slice[slice_offset..slice_offset_end])
                    .expect("read page error");
                offset += write_size;
                slice_offset += write_size;
                total_write_size += write_size;
            }
        }
        if self.file_size() < offset {
            self.set_file_size(offset);
        }
        total_write_size
    }
    fn write_from_kspace(&self, data: &Vec<u8>) -> usize {
        // with page cache
        let mut total_write_size = 0usize;
        let page_cache = self.page_cache().as_ref().cloned().unwrap();
        let mut offset = if self.flags().contains(OpenFlags::O_APPEND) {
            self.file_size()
        } else {
            self.offset()
        };
        let mut slice_offset = 0;
        let slice_end = data.len();
        while slice_offset < slice_end {
            // to avoid slice's length spread page boundary (howerver, it's low probability)
            let page = page_cache.get_page(offset, None).expect("get page error");
            let page_offset = offset % PAGE_SIZE;
            let mut slice_offset_end = slice_offset + (PAGE_SIZE - page_offset);
            if slice_offset_end > slice_end {
                slice_offset_end = slice_end;
            }
            let write_size = page
                .write(page_offset, &data[slice_offset..slice_offset_end])
                .expect("read page error");
            offset += write_size;
            self.seek(offset);
            slice_offset += write_size;
            total_write_size += write_size;
        }
        if self.file_size() < offset {
            self.set_file_size(offset);
        }
        total_write_size
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
        *self.offset.lock()
    }
    fn seek(&self, offset: usize) {
        *self.offset.lock() = offset;
    }
    fn flags(&self) -> OpenFlags {
        *self.flags.lock()
    }
    fn set_flags(&self, flag: OpenFlags) {
        self.flags.lock().set(flag, true);
    }
    fn set_cloexec(&self) {
        *self.available.lock() = false;
    }
    // set dir entry
    fn dirent(&self, dirent: &mut Dirent) -> isize {
        if !self.is_dir() {
            return -1;
        }
        let inner = self.inner();
        let offset = self.offset();
        if let Some((name, offset, first_cluster, _attr)) = inner.dir_info(offset) {
            dirent.init(name.as_str(), offset as isize, first_cluster as usize);
            self.seek(offset as usize);
            // return size of Dirent as read size
            core::mem::size_of::<Dirent>() as isize
        } else {
            -1
        }
    }
    fn fstat(&self, kstat: &mut Kstat) {
        let inner = self.inner();
        let vfile = inner.clone();
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
        let inner = self.inner();
        inner.modify_size(new_length);
    }
}
