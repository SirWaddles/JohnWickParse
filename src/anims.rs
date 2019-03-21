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

    let skeleton_map = get_skeleton_map(&anim)?;
    let track_map = anim.get_track_map();

    Ok(GLTFContainer {
        buffer,
        data: mesh_data,
    })
}

fn get_skeleton_map(anim: &UAnimSequence) -> ParserResult<Vec<String>> {
    let property = anim.get_super_object().get_properties().iter().fold(None, |acc, v| {
        if v.get_name() == "Skeleton" {
            return Some(v);
        }
        acc
    }).unwrap();
    let path = match property.get_data() {
        FPropertyTagType::ObjectProperty(import) => import.get_import(),
        _ => return Err(ParserError::new(format!("Skeleton unreadable format"))),
    };

    let skeleton_path = "skeletons/".to_owned() + path;
    let package = Package::from_file(&skeleton_path)?;
    let skeleton = get_skeleton(package)?;

    let names = skeleton.get_reference().get_bone_info().iter().map(|v| v.get_name().to_owned()).collect();
    Ok(names)
}

fn get_skeleton(package: Package) -> ParserResult<USkeleton> {
    let exports = package.get_exports();
    for export in exports {
        if let Ok(skeleton) = export.downcast::<USkeleton>() {
            return Ok(*skeleton);
        }
    }

    Err(ParserError::new(format!("Skeleton not exported")))
}