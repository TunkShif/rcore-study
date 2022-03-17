use std::{fs, thread, time::Duration};

fn main() {
    thread::sleep(Duration::from_secs(5));
    let text = "hello, world";
    println!("{}", text);
    fs::write("./text.txt", text).expect("failed to write");
}
