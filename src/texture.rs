use std::io::Cursor;
use image::{dxt, ImageDecoder};

pub fn decode_texture(data: &Vec<u8>, width: u32, height: u32) -> Option<Vec<u8>> {
    let reader = Cursor::new(data);
    let decoder = match dxt::DXTDecoder::new(reader, width, height, dxt::DXTVariant::DXT5) {
        Ok(data) => data,
        Err(_) => return None,
    };

    let bytes = match decoder.read_image() {
        Ok(data) => data,
        Err(_) => return None,
    };

    Some(bytes)
}

#[allow(dead_code)]
pub fn save_texture(path: &str, bytes: &Vec<u8>, width: u32, height: u32) {
    image::save_buffer(path, bytes, width, height, image::RGBA(8)).unwrap()
}