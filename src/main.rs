extern crate byteorder;
extern crate hex;

use std::io::Write;
use std::fs::File;

mod rijndael;
mod assets;
mod archives;

fn main() {
    let archive = archives::PakExtractor::new("pakchunk0-WindowsClient.pak");
    let entries = archive.get_entries();
    let file_list = entries.into_iter().map(|v| v.get_filename()).fold(String::new(), |acc, v| acc + v + "\n");
    let mut file = File::create("assets.txt").unwrap();
    file.write_all(file_list.as_bytes()).unwrap();
}
