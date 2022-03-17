#![feature(panic_info_message)]
#![no_std]
#![no_main]

#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    println!("Hello World!");
    log::info!("test");
    log::error!("error!");
    sbi::shutdown();
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    ((sbss as usize)..(ebss as usize)).for_each(|b| unsafe { (b as *mut u8).write_volatile(0) });
}
