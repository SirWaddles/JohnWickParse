use std::path::Path;
use crate::meshes::gltf::*;
use crate::assets::*;

pub fn decode_anim(package: Package, path: &str) -> ParserResult<GLTFContainer> {
    let filepath = Path::new(path);
    let filename = filepath.file_name().unwrap().to_str().unwrap().to_owned();
    
    let exports = package.get_exports();
    for export in exports {
        if let Ok(anim) = export.downcast::<UAnimSequence>() {
            return decode_anim_type(*anim, filename);
        }
    }

    Err(ParserError::new(format!("Package not supported")))
}


fn decode_anim_type(anim: UAnimSequence, asset_name: String) -> ParserResult<GLTFContainer> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut mesh_data = GLTFItem::new();

    Ok(GLTFContainer {
        buffer,
        data: mesh_data,
    })
}