use super::address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::page_table::PTEFlags;
use super::{page_table, PageTable};
use crate::fs::open_flags::CreateMode;
use crate::task::current_task;
use crate::timer::get_time;
use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use nix::ipc::ShmFlags;
use spin::Mutex;
lazy_static! {
    /// 物理页帧管理器实例
    /// - 全局变量，管理除内核空间外的内存空间
    pub static ref SHM_MANAGER: Mutex<SharedMemoryManager> =
        Mutex::new(SharedMemoryManager::new());
}
pub struct SharedMemoryManager {
    shm_areas: BTreeMap<usize, SharedMemoryArea>,
}
pub struct SharedMemoryArea {
    shmid_ds: SharedMemoryIdentifierDs,
    buffer: Vec<u8>,
}
pub struct SharedMemoryIdentifierDs {
    shm_perm: CreateMode, /* Ownership and permissions */
    shm_size: usize,      /* Size of segment (bytes) */
    shm_atime: usize,     /* Last attach time */
    shm_dtime: usize,     /* Last detach time */
    shm_ctime: usize,     /* Creation time/time of last modification via shmctl() */
    shm_cpid: usize,      /* PID of creator */
    shm_lpid: usize,      /* PID of last shmat(2)/shmdt(2) */
    shm_nattch: usize,    /* Number of current attaches */
}
pub struct SharedMemoryTracker {
    pub key: usize,
}
impl SharedMemoryTracker {
    pub fn new(key: usize) -> Self {
        attach_shm(key);
        Self { key }
    }
}
impl Drop for SharedMemoryTracker {
    fn drop(&mut self) {
        detach_shm(self.key);
    }
}
impl SharedMemoryManager {
    pub fn new() -> Self {
        Self {
            shm_areas: BTreeMap::new(),
        }
    }
    pub fn create(&mut self, key: usize, size: usize, shmflags: usize) {
        let pid = current_task().unwrap().pid.0;
        let perm = CreateMode::from_bits((shmflags & 0o777) as u32).unwrap();
        let shmid_ds = SharedMemoryIdentifierDs {
            shm_perm: perm,
            shm_size: size,
            shm_atime: 0,
            shm_dtime: 0,
            shm_ctime: get_time(),
            shm_cpid: pid,
            shm_lpid: 0,
            shm_nattch: 0,
        };
        let buffer: Vec<u8> = vec![0 as u8; size];
        let shm_area = SharedMemoryArea { shmid_ds, buffer };
        self.shm_areas.insert(key, shm_area);
    }
    pub fn attach(&mut self, key: usize) {
        let pid = current_task().unwrap().pid.0;
        let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
        shm_area.shmid_ds.shm_atime = get_time();
        shm_area.shmid_ds.shm_lpid = pid;
        shm_area.shmid_ds.shm_nattch += 1;
    }
    pub fn detach(&mut self, key: usize) {
        let pid = current_task().unwrap().pid.0;
        let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
        shm_area.shmid_ds.shm_dtime = get_time();
        shm_area.shmid_ds.shm_lpid = pid;
        shm_area.shmid_ds.shm_nattch -= 1;
    }
    pub fn remove(&mut self, key: usize) {
        let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
        if shm_area.shmid_ds.shm_nattch == 0 {
            println!("shm remove!");
            self.shm_areas.remove(&key);
        };
    }
    pub fn get_address_and_size(&self, key: usize) -> (PhysAddr, usize) {
        let shm_area = &self.shm_areas.get(&key).unwrap();
        let size = shm_area.shmid_ds.shm_size;
        ((shm_area.buffer.as_ptr() as usize).into(), size)
    }
    pub fn get_nattch(&self, key: usize) -> usize {
        let shm_area = &self.shm_areas.get(&key).unwrap();
        shm_area.shmid_ds.shm_nattch
    }
}

pub fn create_shm(key: usize, size: usize, perm: usize) {
    SHM_MANAGER.lock().create(key, size, perm);
}
pub fn attach_shm(key: usize) {
    SHM_MANAGER.lock().attach(key);
}
pub fn detach_shm(key: usize) {
    SHM_MANAGER.lock().detach(key);
}
pub fn remove_shm(key: usize) {
    SHM_MANAGER.lock().remove(key);
}

pub fn shm_get_address_and_size(key: usize) -> (PhysAddr, usize) {
    SHM_MANAGER.lock().get_address_and_size(key)
}
pub fn shm_get_nattch(key: usize) -> usize {
    SHM_MANAGER.lock().get_nattch(key)
}
