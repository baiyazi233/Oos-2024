//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;
pub use crate::syscall::TaskInfo;
use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};
pub use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr};
pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor, set_current
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    // 取出当前正在执行的任务，修改其进程控制块内的状态
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    // 将这个任务放入任务管理器的队尾
    add_task(task);
    // jump to scheduling cycle
    // 调度并切换任务
    schedule(task_cx_ptr);
}//当仅有一个任务的时候， suspend_current_and_run_next 的效果是会继续执行这个任务

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}


/// 添加一个逻辑段到应用地址空间
pub fn add_maparea(start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission){
    let task = take_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.add_maparea(start_va, end_va, permission);
    drop(inner);
    set_current(task);
}

/// 删除应用地址空间的一个逻辑段
pub fn remove_maparea(start_va: VirtAddr, end_va: VirtAddr) -> isize{
    let task = take_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let i = inner.remove_maparea(start_va, end_va);
    drop(inner);
    set_current(task);
    i
}

/// 检测新的映射区域是否与已有的映射区域冲突
pub fn check_maparea(start_va: VirtAddr, end_va: VirtAddr) -> bool {
    let task = take_current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let i = inner.memory_set.check_conflict(start_va, end_va);
    drop(inner);
    set_current(task);
    i
}

/// update taskinfo
pub fn update_taskinfo(id: usize) -> isize {
    let task = take_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let i = inner.update_taskinfo(id);
    drop(inner);
    set_current(task);
    i
}

/// get taskinfo
pub fn get_taskinfo() -> TaskInfo {
    let task = take_current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let i = inner.get_taskinfo();
    drop(inner);
    set_current(task);
    i
}

/// 检测新的映射区域是否与已有的映射区域冲突
pub fn check_mapsetarea(start_va: VirtAddr, end_va: VirtAddr) -> bool {
    let task = take_current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let i = inner.check_maparea(start_va, end_va);
    drop(inner);
    set_current(task);
    i
}