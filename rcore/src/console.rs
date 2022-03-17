use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct StdOut;

impl Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.chars().for_each(|ch| console_putchar(ch as usize));
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    StdOut.write_fmt(args).unwrap()
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $(arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
