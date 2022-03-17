pub fn print_mem_layout() {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
    }
    log::info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    log::debug!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    log::error!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
}
