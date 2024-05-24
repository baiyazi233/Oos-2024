use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{suspend_current_and_run_next, block_current_and_run_next, current_process, current_task, current_user_token};
use crate::timer::{get_time,add_timer, get_time_ms, NSEC_PER_SEC};
use alloc::sync::Arc;
use crate::config::CLOCK_FREQ;
use crate::mm::{translated_ref, translated_refmut};

/// sleep syscall
pub fn sys_sleep(req: *mut u64) -> isize {
    let token = current_user_token();
    let sec = *translated_ref(token, req);
    let nano_sec = *translated_ref(token, unsafe { req.add(1) });
    let end_time =
        get_time() + sec as usize * CLOCK_FREQ + nano_sec as usize * CLOCK_FREQ / NSEC_PER_SEC;
    loop {
        let current_time = get_time();
        if current_time >= end_time {
            break;
        } else {
            suspend_current_and_run_next()
        }
    }
    0
}
