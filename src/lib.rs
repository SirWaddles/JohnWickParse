#![allow(warnings)]

// use crate::assets::{ParserResult, ParserError, Package, Texture2D, USoundWave};
use crate::assets::{ParserResult, ParserError, Package};

pub mod assets;
pub mod archives;
mod mapping;
mod dispatch;
//mod sound;
mod decompress;
//mod texture;

//pub fn read_asset(asset: &[u8], ubulk: Option<&[u8]>) -> ParserResult<Package> {
//    Package::from_buffer(asset, ubulk)
//}

/*pub fn read_texture(package: Package) -> ParserResult<Vec<u8>> {
    let package_export = package.get_export_move(0)?;
    let texture = match package_export.downcast::<Texture2D>() {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("Export is not texture"))),
    };
    texture::decode_texture(*texture)
}

/// Extracts sounds from a Package struct
pub fn read_sound(package: Package) -> ParserResult<Vec<u8>> {
    let package_export = package.get_export_move(0)?;
    let sound = match package_export.downcast::<USoundWave>() {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("Export is not a sound"))),
    };
    sound::decode_sound(*sound)
}*/