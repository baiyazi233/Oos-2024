//! Implementation of  [`ProcessControlBlock`]

use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::TaskControlBlock;
use super::{add_task, SignalFlags};
use super::{pid_alloc, PidHandle};
use crate::fs::{FdTable, FileDescriptor, File, Stdin, Stdout, ROOT_FD, OpenFlags};
use crate::mm::{translated_refmut, MemorySet, KERNEL_SPACE, VirtAddr, MapPermission};
use crate::sync::{Condvar, Mutex, Semaphore, UPSafeCell};
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;
use spin::Mutex as MutexSpin;
/// Process Control Block
pub struct ProcessControlBlock {
    /// immutable
    pub pid: PidHandle,
    /// mutable
    inner: UPSafeCell<ProcessControlBlockInner>,
    
}

#[derive(Clone)]
pub struct FsStatus {
    pub working_inode: Arc<FileDescriptor>,
}
/// Inner of Process Control Block
pub struct ProcessControlBlockInner {
    /// is zombie?
    pub is_zombie: bool,
    /// memory set(address space)
    pub memory_set: MemorySet,
    /// parent process
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// children process
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// exit code
    pub exit_code: i32,
    /// file descriptor table
    pub fd_table: Arc<MutexSpin<FdTable>>,
    /// 
    pub work_path: Arc<MutexSpin<FsStatus>>,
    /// signal flags
    pub signals: SignalFlags,
    /// tasks(also known as threads)
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    /// task resource allocator
    pub task_res_allocator: RecycleAllocator,
    /// mutex list
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    /// semaphore list
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    /// condvar list
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
    pub heap_base: VirtAddr,
    pub heap_end: VirtAddr,
    pub current_path: String,
}

impl ProcessControlBlockInner {
    #[allow(unused)]
    /// get the address of app's page table
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// allocate a new file descriptor
    // pub fn alloc_fd(&mut self) -> usize {
    //     if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
    //         fd
    //     } else {
    //         self.fd_table.push(None);
    //         self.fd_table.len() - 1
    //     }
    // }
    /// allocate a new task id
    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }
    /// deallocate a task id
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }
    /// the count of tasks(threads) in this process
    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }
    /// get a task with tid in this process
    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }

    /// 添加一个逻辑段到应用地址空间
    pub fn add_maparea(&mut self, start_addr: usize, len: usize, offset: usize, fd: usize) -> isize {
        let file_descriptor = match self.fd_table.lock().get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        };
        let context = file_descriptor.read_all();
        // only support one time mmap becasue we doesn't save mmap_area_end
        // start_addr euqal to MMAP_BASE
        // todo: add a value called mmap_area_end to support multiple mmap
        self.memory_set.map_area(start_addr, len, offset, context)
    }
    /// 删除应用地址空间的一个逻辑段
    pub fn remove_maparea(&mut self, start_va: VirtAddr, end_va: VirtAddr) -> isize {
        self.memory_set.remove_framed_area(start_va, end_va)
    }

    /// 检测新的映射区域是否与已有的映射区域冲突
    pub fn check_maparea(&self, start_va: VirtAddr, end_va: VirtAddr) -> bool {
        self.memory_set.check_conflict(start_va, end_va)
    }


}

impl ProcessControlBlock {
    /// inner_exclusive_access
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// new process from elf file
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        trace!("kernel: ProcessControlBlock::new");
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        // allocate a pid
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: Arc::new(MutexSpin::new(FdTable::new({
                        let mut vec = Vec::with_capacity(144);
                        let stdin = Some(FileDescriptor::new(false, false, Arc::new(Stdin)));
                        let stdout = Some(FileDescriptor::new(false, false, Arc::new(Stdout)));
                        let stderr = Some(FileDescriptor::new(false, false, Arc::new(Stdout)));
                        vec.push(stdin);
                        vec.push(stdout);
                        vec.push(stderr);
                        vec
                    }))),
                    work_path: Arc::new(MutexSpin::new(FsStatus {
                        working_inode: Arc::new(
                            ROOT_FD
                                .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
                                .unwrap(),
                        ),
                    })),
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    heap_base: ustack_base.into(),
                    heap_end: ustack_base.into(),
                    current_path: String::from("/"),
                })
            },
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));
        // prepare trap_cx of main thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>, envp: Vec<String>,) {
        trace!("kernel: exec");
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // memory_set with elf program headers/trampoline/trap context/user stack
        trace!("kernel: exec .. MemorySet::from_elf");
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();
        // substitute memory_set
        trace!("kernel: exec .. substitute memory_set");
        self.inner_exclusive_access().memory_set = memory_set;
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        trace!("kernel: exec .. alloc user resource for main thread again");
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        trace!("kernel: exec .. push arguments on user stack");
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    new_token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(new_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(new_token, p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // initialize trap_cx
        trace!("kernel: exec .. initialize trap_cx");
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        trace!("kernel: fork");
        let mut parent = self.inner_exclusive_access();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table: Vec<Option<FileDescriptor>> = Vec::new();
        for fd in parent.fd_table.lock().iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        let new_fd_table = Arc::new(MutexSpin::new(FdTable::new(new_fd_table)));
        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    work_path: Arc::new(MutexSpin::new(FsStatus {
                        working_inode: Arc::new(
                            ROOT_FD
                                .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
                                .unwrap(),
                        ),
                    })),
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    heap_base: parent.heap_base,
                    heap_end: parent.heap_base,
                    current_path: parent.current_path.clone(),
                })
            },
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        // attach task to child process
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }
    /// get pid
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}

pub const CSIGNAL: usize = 0x000000ff; /* signal mask to be sent at exit */
bitflags! {
    pub struct CloneFlags: u32 {
        const CLONE_VM	            = 0x00000100;/* set if VM shared between processes */
        const CLONE_FS	            = 0x00000200;/* set if fs info shared between processes */
        const CLONE_FILES	        = 0x00000400;/* set if open files shared between processes */
        const CLONE_SIGHAND	        = 0x00000800;/* set if signal handlers and blocked signals shared */
        const CLONE_PIDFD	        = 0x00001000;/* set if a pidfd should be placed in parent */
        const CLONE_PTRACE	        = 0x00002000;/* set if we want to let tracing continue on the child too */
        const CLONE_VFORK	        = 0x00004000;/* set if the parent wants the child to wake it up on mm_release */
        const CLONE_PARENT	        = 0x00008000;/* set if we want to have the same parent as the cloner */
        const CLONE_THREAD	        = 0x00010000;/* Same thread group? */
        const CLONE_NEWNS	        = 0x00020000;/* New mount namespace group */
        const CLONE_SYSVSEM	        = 0x00040000;/* share system V SEM_UNDO semantics */
        const CLONE_SETTLS	        = 0x00080000;/* create a new TLS for the child */
        const CLONE_PARENT_SETTID	= 0x00100000;/* set the TID in the parent */
        const CLONE_CHILD_CLEARTID	= 0x00200000;/* clear the TID in the child */
        const CLONE_DETACHED		= 0x00400000;/* Unused, ignored */
        const CLONE_UNTRACED		= 0x00800000;/* set if the tracing process can't force CLONE_PTRACE on this clone */
        const CLONE_CHILD_SETTID	= 0x01000000;/* set the TID in the child */
        const CLONE_NEWCGROUP		= 0x02000000;/* New cgroup namespace */
        const CLONE_NEWUTS		    = 0x04000000;/* New utsname namespace */
        const CLONE_NEWIPC		    = 0x08000000;/* New ipc namespace */
        const CLONE_NEWUSER		    = 0x10000000;/* New user namespace */
        const CLONE_NEWPID		    = 0x20000000;/* New pid namespace */
        const CLONE_NEWNET		    = 0x40000000;/* New network namespace */
        const CLONE_IO		        = 0x80000000;/* Clone io context */
    }
}