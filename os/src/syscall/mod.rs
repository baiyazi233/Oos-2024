//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

/// fs
pub const SYSCALL_GETCWD: usize = 17;
pub const SYSCALL_PIPE: usize = 59;
pub const SYSCALL_DUP: usize = 23;
pub const SYSCALL_DUP3: usize = 24;
pub const SYSCALL_CHDIR: usize = 49;
pub const SYSCALL_OPENAT: usize = 56;
pub const SYSCALL_CLOSE: usize = 57;
pub const SYSCALL_GETDENTS64: usize = 61;
pub const SYSCALL_READ: usize = 63;
pub const SYSCALL_WRITE: usize = 64;
pub const SYSCALL_LINKAT: usize = 37;
pub const SYSCALL_UNLINKAT: usize = 35;
pub const SYSCALL_MKDIRAT: usize = 34;
pub const SYSCALL_UMOUNT2: usize = 39;
pub const SYSCALL_MOUNT: usize = 40;
pub const SYSCALL_FSTAT: usize = 80;
/// process
pub const SYSCALL_FORK: usize = 220;
pub const SYSCALL_CLONE: usize = 220;
pub const SYSCALL_EXEC: usize = 221;
pub const SYSCALL_WAIT4: usize = 260;
pub const SYSCALL_EXIT: usize = 93;
pub const SYSCALL_GETPPID: usize = 173;
pub const SYSCALL_GETPID: usize = 172;
/// mm
pub const SYSCALL_BRK: usize = 214;
pub const SYSCALL_MUNMAP: usize = 215;
pub const SYSCALL_MMAP: usize = 222;
/// others
pub const SYSCALL_TIMES: usize = 153;
pub const SYSCALL_UNAME: usize = 160;
pub const SYSCALL_YIELD: usize = 124;
pub const SYSCALL_GETTIMEOFDAY: usize = 169;
pub const SYSCALL_SLEEP: usize = 101;

/// kill syscall
pub const SYSCALL_KILL: usize = 129;
/// gettid syscall
pub const SYSCALL_GETTID: usize = 178;
/// spawn syscall
pub const SYSCALL_SPAWN: usize = 400;
/// thread_create syscall
pub const SYSCALL_THREAD_CREATE: usize = 460;
/// waittid syscall
pub const SYSCALL_WAITTID: usize = 462;


// Not standard POSIX sys_call
const SYSCALL_SHUTDOWN: usize = 2000;
const SYSCALL_OPEN: usize = 506;

pub mod errno;
mod fs;
mod process;
mod sync;
mod thread;

use fs::*;
use process::*;
use sync::*;
use thread::*;

use crate::fs::Stat;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYSCALL_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_DUP3 => -1,
        SYSCALL_CHDIR => sys_chdir(args[0] as *const u8),
        SYSCALL_LINKAT => sys_linkat(args[1] as *const u8, args[3] as *const u8),
        SYSCALL_UNLINKAT => sys_unlinkat(args[1] as *const u8),
        SYSCALL_OPEN => sys_openat(AT_FDCWD, args[0] as *const u8, args[1] as u32, 0o777u32),
        SYSCALL_OPENAT => sys_openat(
            args[0],
            args[1] as *const u8,
            args[2] as u32,
            args[3] as u32,
        ),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_GETDENTS64 => sys_getdents64(args[0], args[1] as *mut u8, args[2]),
        //SYSCALL_PIPE => sys_pipe(args[0] as *mut usize),
        SYSCALL_MKDIRAT => -1,
        SYSCALL_UMOUNT2 => -1,
        SYSCALL_MOUNT => -1,
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut u8),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_SLEEP => sys_sleep(args[0] as *mut u64),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_GETTID => sys_gettid(),
        SYSCALL_FORK => sys_fork(args[0], args[1], args[2], args[3], args[4]),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize, args[2] as *const usize,),
        SYSCALL_WAIT4 => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_GETPPID => 1,
        SYSCALL_GETTIMEOFDAY => sys_get_time(args[0] as *mut usize),
        SYSCALL_BRK => sys_brk(args[0]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_TIMES => sys_times(args[0] as *mut usize),
        SYSCALL_UNAME => -1,
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        SYSCALL_THREAD_CREATE => sys_thread_create(args[0], args[1]),
        SYSCALL_WAITTID => sys_waittid(args[0]) as isize,
        SYSCALL_KILL => sys_kill(args[0], args[1] as u32),
        SYSCALL_SHUTDOWN => sys_shutdown(args[0] != 0),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }

}
