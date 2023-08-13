// ipc.h

// const IPC_CREAT = 0o1000;   /* create if key is nonexistent */
// const IPC_EXCL  = 0o2000;   /* fail if key exists */
// const IPC_NOWAIT =0o4000;   /* return error on wait */

/*
 * Control commands used with semctl, msgctl and shmctl
 * see also specific commands in sem.h, msg.h and shm.h
 */

pub const IPC_PRIVATE: usize = 0;
pub const IPC_RMID: usize = 0; /* remove resource */
pub const IPC_SET: usize = 1; /* set ipc_perm options */
pub const IPC_STAT: usize = 2; /* get ipc_perm options */
pub const IPC_INFO: usize = 3; /* see ipcs */

bitflags! {
    #[derive(Debug)]
    pub struct ShmFlags: u32 {
    const IPC_CREAT = 0o1000;   /* create if key is nonexistent */
    const IPC_EXCL  = 0o2000;   /* fail if key exists */
    const IPC_NOWAIT =0o4000;   /* return error on wait */

    }
}
// struct ipc_perm
// {
//         __kernel_key_t        key;
//         __kernel_uid_t        uid;
//         __kernel_gid_t        gid;
//         __kernel_uid_t        cuid;
//         __kernel_gid_t        cgid;
//         __kernel_mode_t        mode;
//         unsigned short        seq;
// };
