use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use errno::Errno;
use hashbrown::HashMap;
use nix::TimeSpec;
use spin::{Lazy, RwLock};

use crate::mm::translated_ref;

use crate::syscall::errno;
use crate::syscall::futex::{FutexQueue, FutexWaiter};
use crate::task::{
    block_current_and_run_next, current_task, current_user_token, suspend_current_and_run_next,
    unblock_task, TaskControlBlock,
};
use crate::timer::{get_time_us, USEC_PER_SEC};

use super::Result;

const FUTEX_WAIT: usize = 0;
const FUTEX_WAKE: usize = 1;
const FUTEX_REQUEUE: usize = 3;

const FUTEX_PRIVATE_FLAG: usize = 128;
const FUTEX_CLOCK_REALTIME: usize = 256;
const FUTEX_CMD_MASK: usize = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

lazy_static! {
    pub static ref FUTEX_QUEUE: RwLock<HashMap<usize, FutexQueue>> = RwLock::new(HashMap::new());
}

/// Argument:
/// - uaddr: futex address, uaddr points to the futex word
/// - futex_op: futex operation
/// - val: value
/// - timeout/val2: timeout
/// - uaddr2: futex address 2
/// - val3: value 3
pub fn sys_futex(
    uaddr: *const u32,
    futex_op: usize,
    val: u32,
    val2: *const u32,
    uaddr2: *const u32,
    val3: u32,
) -> Result<i32> {
    let option = futex_op & FUTEX_CMD_MASK;
    let token = current_user_token();
    println!(
        "*****sys_futex(uaddr: {:#x?}, futex_op: {:x?}, val: {:x?}, timeout: {:#x?}, uaddr2: {:#x?}, val3: {:x?}) = ?",
        uaddr,
        futex_op,
        val,
        val2,
        uaddr2,
        val3,
    );
    if futex_op & FUTEX_CLOCK_REALTIME != 0 {
        if option != FUTEX_WAIT {
            // return Err(-EPERM); // ENOSYS
            return Err(Errno::EPERM);
        }
    }
    let ret = match option {
        FUTEX_WAIT => {
            // val2 is a timespec
            let time = if val2 as usize != 0 {
                let ts = translated_ref(token, val2 as *const TimeSpec);
                ts.into_ticks()
            } else {
                usize::MAX // inf
            };
            futex_wait(uaddr as usize, val, time)
        }
        FUTEX_WAKE => futex_wake(uaddr as usize, val),
        FUTEX_REQUEUE => {
            // val2 is a limit
            futex_requeue(uaddr as usize, val, uaddr2 as usize, val2 as u32)
        }
        _ => panic!("ENOSYS"),
    };
    println!(
        "sys_futex(uaddr: {:#x?}, futex_op: {:x?}, val: {:x?}, timeout: {:#x?}, uaddr2: {:#x?}, val3: {:x?}) = {:?}",
        uaddr,
        futex_op,
        val,
        val2,
        uaddr2,
        val3,
        ret,
    );
    ret
}

/// 测试地址uaddr指向的futex字中的值是否仍然包含期望的值val，如果是，则等待futex词上的FUTEX_WAKE操作
pub fn futex_wait(uaddr: usize, val: u32, timeout: usize) -> Result<i32> {
    // futex_wait_setup
    let mut fq_writer = FUTEX_QUEUE.write();
    let flag = fq_writer.contains_key(&uaddr);
    let fq = if flag {
        fq_writer.get(&uaddr).unwrap()
    } else {
        fq_writer.insert(uaddr, FutexQueue::new());
        fq_writer.get(&uaddr).unwrap()
    };
    fq.waiters_increase();
    let mut fq_lock = fq.chain.write();
    let token = current_user_token();
    let uval = translated_ref(token, uaddr as *const AtomicU32);
    // debug!(
    //     "futex_wait: uval: {:x?}, val: {:x?}, timeout: {}",
    //     uval, val, timeout
    // );
    // Ordering is Relaxed
    if uval.load(Ordering::Relaxed) != val {
        drop(fq_lock);
        fq.waiters_decrease();
        if fq.waiters() == 0 {
            fq_writer.remove(&uaddr);
        }
        drop(fq_writer);
        return Err(Errno::EAGAIN);
    }

    // futex_wait_queue_me
    let task = current_task().unwrap();
    fq_lock.push_back(FutexWaiter::new(task.clone(), get_time_us(), timeout));
    drop(fq_lock);
    drop(fq_writer);

    // warning: Auto waking-up has not been implemented yet
    block_current_and_run_next();
    Ok(0)
}

/// 唤醒等待在地址uaddr指向的futex字上的nr_wake个任务
pub fn futex_wake(uaddr: usize, nr_wake: u32) -> Result<i32> {
    let mut fq_writer = FUTEX_QUEUE.write();
    if !fq_writer.contains_key(&uaddr) {
        return Ok(0);
    }
    let fq = fq_writer.get(&uaddr).unwrap();
    let mut fq_lock = fq.chain.write();
    let waiters = fq.waiters();
    if waiters == 0 {
        return Ok(0);
    }
    let nr_wake = nr_wake.min(waiters as u32);
    // debug!("futex_wake: uaddr: {:x?}, nr_wake: {:x?}", uaddr, nr_wake);

    let mut wakeup_queue = Vec::with_capacity(20);
    (0..nr_wake as usize).for_each(|_| {
        // 加入唤醒队列中，但需要等到释放完锁之后才能唤醒
        let task = fq_lock.pop_front().unwrap().task;
        wakeup_queue.push(task);
        fq.waiters_decrease();
    });
    drop(fq_lock);

    if fq.waiters() == 0 {
        fq_writer.remove(&uaddr);
    }

    for task in wakeup_queue.into_iter() {
        unblock_task(task);
    }
    Ok(nr_wake as i32)
}

/// 最多唤醒等待在 uaddr 上的 futex 的 val 个等待者。
/// 如果等待者数量超过了 val，则剩余的等待者将从源 futex 的等待队列中删除，并添加到目标 futex 在 uaddr2 上的等待队列中。
/// val2 参数指定了重新加入到 uaddr2 上的 futex 的等待者的上限数量。
pub fn futex_requeue(uaddr: usize, nr_wake: u32, uaddr2: usize, nr_limit: u32) -> Result<i32> {
    let mut fq_writer = FUTEX_QUEUE.write();
    if !fq_writer.contains_key(&uaddr) {
        return Ok(0);
    }
    let fq = fq_writer.get(&uaddr).unwrap();
    let mut fq_lock = fq.chain.write();
    let waiters = fq.waiters();
    if waiters == 0 {
        return Ok(0);
    }
    let nr_wake = nr_wake.min(waiters as u32);

    let mut wakeup_q = Vec::with_capacity(20);
    let mut requeue_q = Vec::with_capacity(20);

    (0..nr_wake as usize).for_each(|_| {
        // prepare to wake-up
        let task = fq_lock.pop_front().unwrap().task;
        wakeup_q.push(task);
        fq.waiters_decrease();
    });

    let nr_limit = nr_limit.min(fq.waiters() as u32);
    (0..nr_limit as usize).for_each(|_| {
        // prepare to requeue
        let task = fq_lock.pop_front().unwrap();
        requeue_q.push(task);
        fq.waiters_decrease();
    });
    drop(fq_lock);

    // wakeup sleeping tasks
    if fq.waiters() == 0 {
        fq_writer.remove(&uaddr);
    }
    for task in wakeup_q.into_iter() {
        unblock_task(task);
    }

    // requeue...
    if nr_limit == 0 {
        return Ok(nr_wake as i32);
    }

    let flag2 = fq_writer.contains_key(&uaddr2);
    let fq2 = if flag2 {
        fq_writer.get(&uaddr2).unwrap()
    } else {
        fq_writer.insert(uaddr2, FutexQueue::new());
        fq_writer.get(&uaddr2).unwrap()
    };

    let mut fq2_lock = fq2.chain.write();

    for task in requeue_q.into_iter() {
        fq2_lock.push_back(task);
        fq2.waiters_increase();
    }

    Ok(nr_wake as i32)
}
