use std::io::Cursor;
use image::{dxt, png, ImageDecoder, ImageError, ImageBuffer, Bgra, Rgba, ConvertBuffer};
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

pub fn decode_texture(texture: Texture2D) -> ParserResult<Vec<u8>> {
    let pixel_format = texture.get_pixel_format()?.to_owned();
    let texture_mipmap = texture.get_texture_move()?;

    let data = TextureData {
        width: texture_mipmap.get_width(),
        height: texture_mipmap.get_height(),
        data: texture_mipmap.get_bytes_move(),
    };

    let bytes: Vec<u8> = match pixel_format.as_ref() {
        "PF_DXT5" => decode_texture_dxt5(&data)?,
        "PF_DXT1" => decode_texture_dxt1(&data)?,
        "PF_B8G8R8A8" => data.data,
        _ => return Err(ParserError::new(format!("Unsupported pixel format: {}", pixel_format))),
    };

    let colour_type = match pixel_format.as_ref() {
        "PF_DXT1" => image::RGB(8),
        "PF_B8G8R8A8" => image::RGBA(8),
        _ => image::RGBA(8),
    };

    let image_buffer = match pixel_format.as_ref() {
        "PF_DXT1" => {
            bytes
        },
        "PF_DXT5" => {
            bytes
        },
        "PF_B8G8R8A8" => {
            let buf = ImageBuffer::<Bgra<u8>, Vec<u8>>::from_raw(data.width, data.height, bytes).unwrap();
            let rgba_buf: ImageBuffer<Rgba<u8>, Vec<u8>> = buf.convert();
            rgba_buf.into_raw()
        },
        _ => return Err(ParserError::new(format!("Unsupported pixel format: {}", pixel_format))),
    };

    let mut png_data: Vec<u8> = Vec::new();

    let encoder = png::PNGEncoder::new(&mut png_data);
    match encoder.encode(&image_buffer, data.width, data.height, colour_type) {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("PNG conversion failed"))),
    };

    Ok(png_data)
}

fn decode_texture_dxt5(data: &TextureData) -> ParserResult<Vec<u8>> {
    let reader = Cursor::new(&data.data);
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