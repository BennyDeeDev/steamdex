use std::{fs, path::PathBuf};

#[derive(Clone, Copy)]
enum Mode {
    Object,
    String,
    Int,
}

enum Value {
    Object,
    String(String),
    Int(i32),
}

pub fn read_vdf(path: &PathBuf) {
    let raw_bytes = fs::read(path).expect("could not read VDF");

    let mut mode = Mode::Object;
    let mut pos: usize = 0;
    while pos < raw_bytes.len() {
        let byte = raw_bytes[pos];
        pos += 1;

        match (byte, mode) {
            (0x00, Mode::Object) => {}
            _ => {}
        }
    }
}
