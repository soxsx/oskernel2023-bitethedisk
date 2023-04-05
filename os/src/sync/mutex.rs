use core::{
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    marker::PhantomData,
    ops::Deref,
    ops::DerefMut,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::{collections::VecDeque, sync::Arc};

pub type Mutex<T> = SpinMutex<T>;

/// 自旋锁（当前 os 中锁的默认实现）
///
/// 未获得锁的进程将进入自旋状态，直到获得锁
#[repr(C)]
pub struct SpinMutex<T: ?Sized> {
    /// 是否上锁
    locked: AtomicBool,

    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    pub fn new(val: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(val),
        }
    }

    /// 获得锁之前会一直自旋
    pub fn lock(&self) -> MutexGuard<T> {
        while self.locked.load(Ordering::SeqCst) {}

        MutexGuard { lock: self }
    }
}

impl<T: ?Sized> SpinMutex<T> {
    pub fn unlock(&self) {
        // 未上锁
        if !self.locked.load(Ordering::SeqCst) {
            panic!("try unlock on an unlocked mutex!");
        }
        self.locked.store(false, Ordering::SeqCst);
    }
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
