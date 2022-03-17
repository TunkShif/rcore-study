use std::fs;

fn main() {
    fs::read_dir(".")
        .expect("failed to read files")
        .for_each(|dir| println!("{}", dir.unwrap().path().display()));
}
