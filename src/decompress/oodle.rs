extern crate libloading as lib;

type DecompressFunc = unsafe fn(*mut u8, u64, *mut u8, u64, u32, u32, u64, u32, u32, u32, u32, u32, u32, u32) -> i32;

pub fn decompress_stream(uncompressed_size: u64, bytes: &mut [u8]) -> lib::Result<Vec<u8>> {
    let library = lib::Library::new("./oo2core_3_win64.dll")?;
    let mut output = vec![0u8; uncompressed_size as usize];
    let mut check = 0;
    println!("test: {:?}", bytes.len());
    unsafe {
        let func: lib::Symbol<DecompressFunc> = library.get(b"OodleLZ_Decompress")?;
        check = func(bytes.as_mut_ptr(), bytes.len() as u64, output.as_mut_ptr(), uncompressed_size, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3);
    }
    println!("check: {} {}", uncompressed_size, check);
    Ok(output)
}