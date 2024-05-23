use crate::fs::*;
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_process, current_task, current_user_token};
use alloc::sync::Arc;
use super::errno::*;
use core::mem::size_of;
pub const AT_FDCWD: usize = 100usize.wrapping_neg();
/// write syscall
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };
    if !file_descriptor.writable() {
        return EBADF;
    }
    file_descriptor.write_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    ) as isize
}
/// read syscall
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };
    if !file_descriptor.readable() {
        return EBADF;
    }
    file_descriptor.read_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    ) as isize
}

pub fn sys_openat(dirfd: usize, path: *const u8, flags: u32, mode: u32) -> isize {
    let process = current_process();
    let token = current_user_token();
    let path = translated_str(token, path);
    let flags = match OpenFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            warn!("[sys_openat] unknown flags");
            return EINVAL;
        }
    };
    let mode = StatMode::from_bits(mode);
    info!(
        "[sys_openat] dirfd: {}, path: {}, flags: {:?}, mode: {:?}",
        dirfd as isize, path, flags, mode
    );
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let file_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => {
            match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            }
        }
    };
    let new_file_descriptor = match file_descriptor.open(&path, flags, false) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };

    let new_fd = match fd_table.insert(new_file_descriptor) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    new_fd as isize
}
/// open sys
// pub fn sys_open(path: *const u8, flags: u32) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_open",
//         current_task().unwrap().process.upgrade().unwrap().getpid()
//     );
//     let process = current_process();
//     let token = current_user_token();
//     let path = translated_str(token, path);
//     if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
//         let mut inner = process.inner_exclusive_access();
//         let fd = inner.alloc_fd();
//         inner.fd_table[fd] = Some(inode);
//         fd as isize
//     } else {
//         -1
//     }
// }
/// close syscall
pub fn sys_close(fd: usize) -> isize {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    match fd_table.remove(fd) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
    }
}
/// pipe syscall
// pub fn sys_pipe(pipe: *mut usize) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_pipe",
//         current_task().unwrap().process.upgrade().unwrap().getpid()
//     );
//     let process = current_process();
//     let token = current_user_token();
//     let mut inner = process.inner_exclusive_access();
//     let mut fd_table = inner.fd_table.lock();
//     let (pipe_read, pipe_write) = make_pipe();
//     let read_fd = inner.alloc_fd();
//     inner.fd_table[read_fd] = Some(pipe_read);
//     let write_fd = inner.alloc_fd();
//     inner.fd_table[write_fd] = Some(pipe_write);
//     *translated_refmut(token, pipe) = read_fd;
//     *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
//     0
// }
/// dup syscall
pub fn sys_dup(fd: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_dup",
        current_task().unwrap().process.upgrade().unwrap().getpid()
    );
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let fd_file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    let nfd = match fd_table.insert(fd_file_descriptor) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    nfd as isize
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, buf: *mut u8) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();

    let mut fd_table = inner.fd_table.lock();
    let file_descriptor = match fd {
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };

    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        core::mem::size_of::<Stat>(),
    ));
    user_buf.write(file_descriptor.get_stat().as_bytes());
    SUCCESS
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().process.upgrade().unwrap().getpid()
    );
    -1
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().process.upgrade().unwrap().getpid()
    );
    -1
}

pub fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    let process = current_process();
    let token = current_user_token();
    if size == 0 {
        //&& buf != 0
        // The size argument is zero and buf is not a NULL pointer.
        return EINVAL;
    }
    let working_dir = process
        .inner_exclusive_access()
        .work_path
        .working_inode
        .get_cwd()
        .unwrap();
    // println!("[sys_getcwd] cwd = {}",working_dir);
    if working_dir.len() >= size {
        // The size argument is less than the length of the absolute pathname of the working directory,
        // including the terminating null byte.
        return ERANGE;
    }
    let mut userbuf = UserBuffer::new(translated_byte_buffer(token, buf, size));
    let ret = userbuf.write(working_dir.as_bytes());
    if ret == 0 {
        0
    } else {
        buf as isize
    }
}

pub fn sys_chdir(path: *const u8) -> isize {
    let task = current_task().unwrap();
    let token = task.get_user_token();
    let path = translated_str(token, path);
    info!("[sys_chdir] path: {}", path);

    let mut lock = task.fs.lock();

    match lock.working_inode.cd(&path) {
        Ok(new_working_inode) => {
            lock.working_inode = new_working_inode;
            SUCCESS
        }
        Err(errno) => errno,
    }
}

pub fn sys_getdents64(fd: usize, buf: *mut u8, count: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();

    let file_descriptor = match fd {
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };
    let dirent_vec = match file_descriptor.get_dirent(count) {
        Ok(vec) => vec,
        Err(errno) => return errno,
    };
    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        dirent_vec.len() * size_of::<Dirent>(),
    ));
    let buffer_index = dirent_vec.len().min(count / core::mem::size_of::<Dirent>());
    for index in 0..buffer_index {
        user_buf.write_at(size_of::<Dirent>() * index, dirent_vec[index].as_bytes());
    }

    (dirent_vec.len() * size_of::<Dirent>()) as isize
}