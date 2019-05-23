extern crate libloading as lib;

type DecompressFunc = unsafe fn(*const u8, u64, *mut u8, u64, u32, u32, u32, u64, u64, u64, u64, u64, u64, u32) -> i32;

pub fn decompress_stream(uncompressed_size: u64, bytes: &[u8]) -> lib::Result<Vec<u8>> {
    let library = lib::Library::new("./oo2core_5_win64.dll")?;
    let mut output = vec![0u8; uncompressed_size as usize];
    let mut check = 0;
    unsafe {
        let func: lib::Symbol<DecompressFunc> = library.get(b"OodleLZ_Decompress")?;
        check = func(bytes.as_ptr(), bytes.len() as u64, output.as_mut_ptr(), uncompressed_size, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    }
    if uncompressed_size as i32 != check {
        // throw an error, work it out later 
        println!("Compression failure: {} {}", uncompressed_size, check);
    }
    Ok(output)
}