#![allow(dead_code)]

use lazy_static::lazy_static;
use crate::assets::{ParserResult, ParserError, Package, Texture2D};

pub mod assets;
pub mod archives;
pub mod dispatch;
pub mod decompress;
mod mapping;
//mod sound;
mod texture;

lazy_static! {
    static ref GLOBAL_DATA: dispatch::LoaderGlobalData = {
        let mut dispatch = dispatch::Extractor::new("paks/global", None).expect("Could not read global");
        dispatch.read_global().expect("Could not parse global")
    };
}

pub fn read_asset(asset: &[u8], ubulk: Option<&[u8]>) -> ParserResult<Package> {
    Package::from_buffer(asset, ubulk, &GLOBAL_DATA)
}

pub fn read_asset_from_file(file: &str) -> ParserResult<Package> {
    Package::from_file(file, &GLOBAL_DATA)
}

pub fn read_texture(package: Package) -> ParserResult<Vec<u8>> {
    let package_export = package.get_export_move(0)?.into_any();
    let texture = match package_export.downcast::<Texture2D>() {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("Export is not texture"))),
    };
    texture::decode_texture(*texture)
}

/*/// Extracts sounds from a Package struct
pub fn read_sound(package: Package) -> ParserResult<Vec<u8>> {
    let package_export = package.get_export_move(0)?;
    let sound = match package_export.downcast::<USoundWave>() {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("Export is not a sound"))),
    };
    sound::decode_sound(*sound)
}*/