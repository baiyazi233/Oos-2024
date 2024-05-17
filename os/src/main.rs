#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(string_remove_matches)]
#[macro_use]
extern crate log;

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
pub mod config;
pub mod drivers;
pub mod fs;
pub mod lang_items;
pub mod logging;
pub mod mm;
pub mod sbi;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_initial_apps.S"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
/// the rust entry-point of os
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    logging::init();
    mm::init();
    mm::remap_test();
    trap::init();
    println!("[kernel] Finish trap init! ");
    trap::enable_timer_interrupt();
    println!("[kernel] Finish enable timer interrupt! ");
    timer::set_next_trigger();
    println!("[kernel] Finish set trigger! ");
    fs::directory_tree::init_fs();
    println!("[kernel] Finish init fs! ");
    task::load_initialproc();
    println!("[kernel] Finish load initialproc! ");
    task::add_initproc();
    println!("[kernel] Finish add initproc! ");
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
