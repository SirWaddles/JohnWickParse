use libloading as lib;
use lazy_static::lazy_static;

type DecompressFunc = unsafe fn(*const u8, u64, *mut u8, u64, u32, u32, u32, u64, u64, u64, u64, u64, u64, u32) -> i32;

fn get_lib_func() -> lib::Library {
    if cfg!(windows) {
        lib::Library::new("./oo2core_8_win64.dll").unwrap()
    } else {
        lib::Library::new("./oo2core_8_win64.so").unwrap()
    }
}

lazy_static! {
    static ref OODLE: lib::Library = get_lib_func();
}

pub fn decompress_stream(uncompressed_size: u64, bytes: &[u8]) -> lib::Result<Vec<u8>> {
    let mut output = vec![0u8; uncompressed_size as usize];
    let check;
    unsafe {
        let func: lib::Symbol<DecompressFunc> = OODLE.get(b"OodleLZ_Decompress")?;
        check = func(bytes.as_ptr(), bytes.len() as u64, output.as_mut_ptr(), uncompressed_size, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    }
    if uncompressed_size as i32 != check {
        // throw an error, work it out later 
        println!("Compression failure: {} {}", uncompressed_size, check);
    }
    Ok(output)
}