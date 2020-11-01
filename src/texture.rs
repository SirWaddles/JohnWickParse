use std::io::{Cursor, Read};
use byteorder::{ReadBytesExt};
use image::{ImageDecoder, ImageError, ImageBuffer, Bgra, Rgba, buffer::ConvertBuffer, ColorType};
use image::codecs::{dxt, png};
use bitreader::BitReader;
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

    let width = data.width;
    let height = data.height;

    let bytes: Vec<u8> = match pixel_format.as_ref() {
        "PF_DXT5" => decode_texture_dxt5(data)?,
        "PF_DXT1" => decode_texture_dxt1(data)?,
        "PF_B8G8R8A8" => decode_texture_bgra(data)?,
        "PF_BC5" => create_rgb_from_bc5(data.data, data.width, data.height),
        "PF_G8" => data.data,
        _ => return Err(ParserError::new(format!("Unsupported pixel format: {}", pixel_format))),
    };

    let colour_type = match pixel_format.as_ref() {
        "PF_DXT1" => ColorType::Rgb8,
        "PF_B8G8R8A8" => ColorType::Rgba8,
        "PF_BC5" => ColorType::Rgb8,
        "PF_G8" => ColorType::L8,
        _ => ColorType::Rgba8,
    };

    let mut png_data: Vec<u8> = Vec::new();

    let encoder = png::PngEncoder::new(&mut png_data);
    match encoder.encode(&bytes, width, height, colour_type) {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("PNG conversion failed"))),
    };

    Ok(png_data)
}

fn decode_texture_bgra(data: TextureData) -> ParserResult<Vec<u8>> {
    let buf = ImageBuffer::<Bgra<u8>, Vec<u8>>::from_raw(data.width, data.height, data.data).unwrap();
    let rgba_buf: ImageBuffer<Rgba<u8>, Vec<u8>> = buf.convert();
    Ok(rgba_buf.into_raw())
}

fn decode_texture_dxt5(data: TextureData) -> ParserResult<Vec<u8>> {
    let reader = Cursor::new(data.data);
    let decoder = dxt::DxtDecoder::new(reader, data.width, data.height, dxt::DxtVariant::DXT5)?;
    let mut buf = vec![0u8; decoder.total_bytes() as usize];

    decoder.read_image(&mut buf)?;

    Ok(buf)
}

fn decode_texture_dxt1(data: TextureData) -> ParserResult<Vec<u8>> {
    let reader = Cursor::new(data.data);
    let decoder = dxt::DxtDecoder::new(reader, data.width, data.height, dxt::DxtVariant::DXT1)?;
    let mut buf = vec![0u8; decoder.total_bytes() as usize];

    decoder.read_image(&mut buf)?;

    Ok(buf)
}

#[allow(dead_code)]
pub fn save_texture(path: &str, bytes: &Vec<u8>, width: u32, height: u32) {
    image::save_buffer(path, bytes, width, height, ColorType::Rgba8).unwrap()
}

fn get_pixel_loc(width: u32, x: usize, y: usize, off: usize) -> usize {
    (y * (width as usize) + x) * 3 + off
}

fn create_rgb_from_bc5(bytes: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    let mut res = vec![0u8;(width * height * 3) as usize];
    let mut cursor = Cursor::new(bytes);
    for y_block in 0..((height as usize) / 4) {
        for x_block in 0..((width as usize) / 4) {
            let r_bytes = decode_bc3_block(&mut cursor).unwrap();
            let g_bytes = decode_bc3_block(&mut cursor).unwrap();

            for r in 0..16 {
                let x_off = r % 4;
                let y_off = r / 4;
                res[get_pixel_loc(width, x_block * 4 + x_off, y_block * 4 + y_off, 0)] = r_bytes[r];
            }
            for g in 0..16 {
                let x_off = g % 4;
                let y_off = g / 4;
                res[get_pixel_loc(width, x_block * 4 + x_off, y_block * 4 + y_off, 1)] = g_bytes[g];
            }
            for b in 0..16 {
                let x_off = b % 4;
                let y_off = b / 4;
                let b_val = get_z_normal(r_bytes[b], g_bytes[b]);
                res[get_pixel_loc(width, x_block * 4 + x_off, y_block * 4 + y_off, 2)] = b_val;
            }
        }
    }

    res
}

fn get_z_normal(x: u8, y: u8) -> u8 {
    let xf = ((x as f32) / 127.5) - 1.0;
    let yf = ((y as f32) / 127.5) - 1.0;
    let zval = (1.0 - xf*xf - yf*yf).max(0.0).sqrt().min(1.0);
    ((zval * 127.0) + 128.0) as u8 
}

fn decode_bc3_block(buf_in: &mut Cursor<Vec<u8>>) -> ParserResult<[u8;16]> {
    let ref0 = buf_in.read_u8()?;
    let ref1 = buf_in.read_u8()?;
    let ref0 = ref0 as f32;
    let ref1 = ref1 as f32;

    let mut ref_sl = [0f32; 8];
    ref_sl[0] = ref0;
    ref_sl[1] = ref1;

    if ref0 > ref1 {
        ref_sl[2] = (6.0 * ref0 + 1.0 * ref1) / 7.0;
        ref_sl[3] = (5.0 * ref0 + 2.0 * ref1) / 7.0;
        ref_sl[4] = (4.0 * ref0 + 3.0 * ref1) / 7.0;
        ref_sl[5] = (3.0 * ref0 + 4.0 * ref1) / 7.0;
        ref_sl[6] = (2.0 * ref0 + 5.0 * ref1) / 7.0;
        ref_sl[7] = (1.0 * ref0 + 6.0 * ref1) / 7.0;
    } else {
        ref_sl[2] = (4.0 * ref0 + 1.0 * ref1) / 5.0;
        ref_sl[3] = (3.0 * ref0 + 2.0 * ref1) / 5.0;
        ref_sl[4] = (2.0 * ref0 + 3.0 * ref1) / 5.0;
        ref_sl[5] = (1.0 * ref0 + 4.0 * ref1) / 5.0;
        ref_sl[6] = 0.0;
        ref_sl[7] = 255.0;
    }

    let mut index_block1 = [0u8;3];
    buf_in.read_exact(&mut index_block1).unwrap();
    let index_block1 = get_bc3_indices(&index_block1);

    let mut index_block2 = [0u8;3];
    buf_in.read_exact(&mut index_block2).unwrap();
    let index_block2 = get_bc3_indices(&index_block2);

    let mut bytes = [0u8;16];
    for i in 0..8 {
        bytes[7 - i] = ref_sl[index_block1[i as usize] as usize] as u8;
    }
    for i in 0..8 {
        bytes[15 - i] = ref_sl[index_block2[i as usize] as usize] as u8;
    }

    Ok(bytes)
}

fn get_bc3_indices(buf_block: &[u8;3]) -> [u8;8] {
    let mut buf_test = [0u8;3];
    buf_test[0] = buf_block[2];
    buf_test[1] = buf_block[1];
    buf_test[2] = buf_block[0];
    let mut reader = BitReader::new(&buf_test);
    let mut indices = [0u8; 8];

    for i in 0..8 {
        indices[i] = reader.read_u8(3).unwrap();
    }

    indices
}