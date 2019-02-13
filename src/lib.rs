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

/// Extracts a raw RGBA texture from a Package struct 
pub fn read_texture(package: &Package) -> ParserResult<Vec<u8>> {
    let texture = match package.get_export(0)?.downcast_ref::<Texture2D>() {
        Some(data) => data,
        None => return Err(ParserError::new(format!("Package does not export texture"))),
    };

    texture::decode_texture(texture)
}