use std::io::{Cursor, Seek, SeekFrom};
use byteorder::{LittleEndian, WriteBytesExt};
use std::path::Path;
use crate::assets::{USkeletalMesh, ParserResult, ParserError, Package, FVector, FMultisizeIndexContainer};
mod gltf;
use gltf::*;


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

    let lod = mesh.get_first_lod();
    let position_verts = lod.get_position_buffer().get_verts();
    let position_size = write_verts_buffer(position_verts, &mut buffer)?;
    let position_buffer_view = GLTFBufferView::new(0, position_size);
    let buffer_view = mesh_data.add_buffer_view(position_buffer_view);

    let vert_minimum = get_vert_minimum(&position_verts)?;
    let vert_maximum = get_vert_maximum(&position_verts)?;
    let mesh_accessor = GLTFAccessor::new(buffer_view, GLTFComponentType::Float, position_verts.len() as u32, "VEC3", vert_minimum, vert_maximum);
    let mesh_accessor = mesh_data.add_accessor(mesh_accessor);

    let indices = lod.get_indices();
    let index_view = match indices {
        FMultisizeIndexContainer::Indices16(data) => {
            let startpos = buffer.len();
            let length = write_u16_buffer(data, &mut buffer)?;
            GLTFBufferView::new(startpos as u32, length)
        },
        FMultisizeIndexContainer::Indices32(data) => {
            let startpos = buffer.len();
            let length = write_u32_buffer(data, &mut buffer)?;
            GLTFBufferView::new(startpos as u32, length)
        },
    };
    let index_view = mesh_data.add_buffer_view(index_view);

    let index_accessor = match indices {
        FMultisizeIndexContainer::Indices16(data) => {
            GLTFAccessor::new(index_view, GLTFComponentType::UnsignedShort, data.len() as u32, "SCALAR", GLTFAccessorValue::None, GLTFAccessorValue::None)
        },
        FMultisizeIndexContainer::Indices32(data) => {
            GLTFAccessor::new(index_view, GLTFComponentType::UnsignedInt, data.len() as u32, "SCALAR", GLTFAccessorValue::None, GLTFAccessorValue::None)
        },
    };
    let index_accessor = mesh_data.add_accessor(index_accessor);

    let mesh_primitive = GLTFPrimitive::new(index_accessor).add_attribute("POSITION", mesh_accessor);
    let mesh_obj = GLTFMesh::new(vec![mesh_primitive]);
    let mesh_obj = mesh_data.add_mesh(mesh_obj);

    let mesh_node = GLTFNode::new().set_mesh(mesh_obj);
    mesh_data.add_node(mesh_node);

    
    let buffer_desc = GLTFBuffer::new(buffer.len() as u32, buffer_name);
    mesh_data.add_buffer(buffer_desc);

    Ok(GLTFContainer {
        buffer,
        data: mesh_data,
    })
}

fn get_vert_maximum(verts: &Vec<FVector>) -> ParserResult<GLTFAccessorValue> {
    let mut vec = verts.get(0).unwrap().get_tuple();

    for vert in verts {
        let comp = vert.get_tuple();
        if comp.0 > vec.0 {
            vec.0 = comp.0;
        }
        if comp.1 > vec.1 {
            vec.1 = comp.1;
        }
        if comp.2 > vec.2 {
            vec.2 = comp.2;
        }
    }

    Ok(GLTFAccessorValue::Vec3Float(vec.0, vec.1, vec.2))
}

fn get_vert_minimum(verts: &Vec<FVector>) -> ParserResult<GLTFAccessorValue> {
    let mut vec = verts.get(0).unwrap().get_tuple();

    for vert in verts {
        let comp = vert.get_tuple();
        if comp.0 < vec.0 {
            vec.0 = comp.0;
        }
        if comp.1 < vec.1 {
            vec.1 = comp.1;
        }
        if comp.2 < vec.2 {
            vec.2 = comp.2;
        }
    }

    Ok(GLTFAccessorValue::Vec3Float(vec.0, vec.1, vec.2))
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

    Ok(verts.len() as u32 * 3 * 4)
}

fn write_u16_buffer(data: &Vec<u16>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();

    for item in data {
        cursor.write_u16::<LittleEndian>(*item)?;
    }

    Ok(data.len() as u32 * 2)
}

fn write_u32_buffer(data: &Vec<u32>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();

    for item in data {
        cursor.write_u32::<LittleEndian>(*item)?;
    }

    Ok(data.len() as u32 * 4)
}