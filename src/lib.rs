use crate::assets::{ParserResult, ParserError, Package, Texture2D};

pub mod assets;
pub mod archives;
mod decompress;
mod texture;

/// Reads an uasset and uexp file into a Package with all of its exports
pub fn read_asset(asset: &[u8], uexp: &[u8], ubulk: Option<&[u8]>) -> ParserResult<Package> {
    Package::from_buffer(asset, uexp, ubulk)
}

/// Extracts a raw RGBA texture from a Package struct 
pub fn read_texture(package: Package) -> ParserResult<Vec<u8>> {
    let package_export = package.get_export_move(0)?;
    let texture = match package_export.downcast::<Texture2D>() {
        Ok(data) => data,
        Err(_) => return Err(ParserError::new(format!("Export is not texture"))),
    };
    texture::decode_texture(*texture)
}