extern crate byteorder;
extern crate hex;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate erased_serde;
extern crate image;

use crate::assets::{ParserResult, ParserError, Package, Texture2D};

pub mod assets;
pub mod archives;
mod texture;
mod rijndael;

/// Reads an uasset and uexp file into a Package with all of its exports
pub fn read_asset(asset: Vec<u8>, uexp: Vec<u8>) -> ParserResult<Package> {
    Package::from_buffer(asset, uexp)
}

/// A simple struct with the data for returning a texture
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

/// Extracts a raw RGBA texture from a Package struct 
pub fn read_texture(package: &Package) -> ParserResult<TextureData> {
    let texture = match package.get_export().downcast_ref::<Texture2D>() {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Package does not export texture"))),
    };

    let pixel_format = match texture.get_pixel_format() {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Could not decode pixel format"))),
    };
    if pixel_format != "PF_DXT5" {
        return Err(ParserError::new(format!("This parser does not support {}", pixel_format)));
    }

    let texture = match texture.get_texture() {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Could not load texture"))),
    };

    let texture_bytes = match texture::decode_texture(texture.get_bytes(), texture.get_width(), texture.get_height()) {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Could not decode texture"))),
    };

    Ok(TextureData {
        data: texture_bytes,
        width: texture.get_width(),
        height: texture.get_height(),
    })
}