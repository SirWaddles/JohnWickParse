use std::io::Cursor;
use image::{dxt, png, ImageDecoder, ImageError};
use crate::assets::{Texture2D, ParserResult, ParserError};

/// A simple struct with the data for returning a texture
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

impl From<ImageError> for ParserError {
    fn from(error: ImageError) -> ParserError {
        ParserError::new(format!("{}", error))
    }
}

pub fn decode_texture(texture: &Texture2D) -> ParserResult<Vec<u8>> {
    let pixel_format = match texture.get_pixel_format() {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Could not find format"))),
    };
    let texture_mipmap = match texture.get_texture() {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Could not find texture"))),
    };

    let data = TextureData {
        width: texture_mipmap.get_width(),
        height: texture_mipmap.get_height(),
        data: texture_mipmap.get_bytes().clone(),
    };

    let bytes: Vec<u8> = match pixel_format {
        "PF_DXT5" => decode_texture_dxt5(&data)?,
        "PF_DXT1" => decode_texture_dxt1(&data)?,
        "PF_B8G8R8A8" => data.data,
        _ => return Err(ParserError::new(format!("Unsupported pixel format: {}", pixel_format))),
    };

    let colour_type = match pixel_format {
        "PF_B8G8R8A8" => image::BGRA(8),
        _ => image::RGBA(8),
    };

    let mut png_data: Vec<u8> = Vec::new();

    let encoder = png::PNGEncoder::new(&mut png_data);
    match encoder.encode(&bytes, data.width, data.height, colour_type) {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("PNG conversion failed"))),
    };

    Ok(png_data)
}

fn decode_texture_dxt5(data: &TextureData) -> ParserResult<Vec<u8>> {
    let reader = Cursor::new(&data.data);
    println!("Size: {} {}", data.width, data.height);
    let decoder = dxt::DXTDecoder::new(reader, data.width, data.height, dxt::DXTVariant::DXT5)?;

    let bytes = decoder.read_image()?;

    Ok(bytes)
}

fn decode_texture_dxt1(data: &TextureData) -> ParserResult<Vec<u8>> {
    let reader = Cursor::new(&data.data);
    let decoder = dxt::DXTDecoder::new(reader, data.width, data.height, dxt::DXTVariant::DXT1)?;

    let bytes = decoder.read_image()?;

    Ok(bytes)
}

#[allow(dead_code)]
pub fn save_texture(path: &str, bytes: &Vec<u8>, width: u32, height: u32) {
    image::save_buffer(path, bytes, width, height, image::RGBA(8)).unwrap()
}