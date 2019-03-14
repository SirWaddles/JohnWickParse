use std::io::{Cursor, Seek, SeekFrom};
use byteorder::{LittleEndian, WriteBytesExt};
use std::path::Path;
use crate::assets::{USkeletalMesh, ParserResult, ParserError, Package, FVector};
mod gltf;
use gltf::{GLTFItem, GLTFBuffer, GLTFBufferView, GLTFAccessor, GLTFComponentType};


pub struct GLTFContainer {
    pub buffer: Vec<u8>,
    pub data: GLTFItem,
}

pub fn decode_mesh(package: Package, path: &str) -> ParserResult<GLTFContainer> {
    let filepath = Path::new(path);
    let filename = filepath.file_name().unwrap().to_str().unwrap().to_owned() + ".bin";
    let package_export = package.get_export_move(0)?;
    if let Ok(mesh) = package_export.downcast::<USkeletalMesh>() {
        return decode_skeletal_mesh(*mesh, filename);
    }

    Err(ParserError::new(format!("Package not supported")))
}

fn decode_skeletal_mesh(mesh: USkeletalMesh, buffer_name: String) -> ParserResult<GLTFContainer> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut mesh_data = GLTFItem::new();


    let position_verts = mesh.get_first_lod().get_position_buffer().get_verts();
    let position_size = write_verts_buffer(position_verts, &mut buffer)?;
    let position_buffer_view = GLTFBufferView::new(0, position_size);
    let buffer_view = mesh_data.add_buffer_view(position_buffer_view);

    let mesh_accessor = GLTFAccessor::new(buffer_view, GLTFComponentType::Float, position_verts.len() as u32, "VEC3");
    mesh_data.add_accessor(mesh_accessor);

    let buffer_desc = GLTFBuffer::new(buffer.len() as u32, buffer_name);
    mesh_data.add_buffer(buffer_desc);

    
    

    Ok(GLTFContainer {
        buffer,
        data: mesh_data,
    })
}

fn write_verts_buffer(verts: &Vec<FVector>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    for i in 0..verts.len() {
        let vert = verts[i].get_tuple();
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
        cursor.write_f32::<LittleEndian>(vert.2)?;
    }

    return Ok(verts.len() as u32 * 3 * 4);
}