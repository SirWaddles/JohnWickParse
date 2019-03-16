use std::io::{Cursor, Seek, SeekFrom};
use byteorder::{LittleEndian, WriteBytesExt};
use std::path::Path;
use crate::assets::*;
mod gltf;
use gltf::*;


pub struct GLTFContainer {
    pub buffer: Vec<u8>,
    pub data: GLTFItem,
}

pub fn decode_mesh(package: Package, path: &str) -> ParserResult<GLTFContainer> {
    let filepath = Path::new(path);
    let filename = filepath.file_name().unwrap().to_str().unwrap().to_owned();
    let package_export = package.get_export_move(0)?;
    if let Ok(mesh) = package_export.downcast::<USkeletalMesh>() {
        return decode_skeletal_mesh(*mesh, filename);
    }

    Err(ParserError::new(format!("Package not supported")))
}

fn decode_skeletal_mesh(mesh: USkeletalMesh, asset_name: String) -> ParserResult<GLTFContainer> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut mesh_data = GLTFItem::new();

    let material = mesh.get_materials()[0].get_interface();
    let material = load_material(&mut mesh_data, material);
    let material = mesh_data.add_material(material);
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

    let tangents = lod.get_static_buffer().get_tangents();
    let tangent_vectors = match tangents {
        FStaticMeshVertexDataTangent::High(_data) => {
            println!("not supported tangent precision");
            Vec::new()
        },
        FStaticMeshVertexDataTangent::Low(data) => {
            data.iter().map(|v| v.get_tangent().get_vector()).collect()
        },
    };
    let tangent_buffer_view = {
        let startpos = buffer.len();
        let length = write_verts_buffer4(&tangent_vectors, &mut buffer)?;
        GLTFBufferView::new(startpos as u32, length)
    };
    let tangent_buffer_view = mesh_data.add_buffer_view(tangent_buffer_view);
    let tangent_accessor = mesh_data.add_accessor(
        GLTFAccessor::new(
            tangent_buffer_view,
            GLTFComponentType::Float, tangent_vectors.len() as u32,
            "VEC4",
            GLTFAccessorValue::None, GLTFAccessorValue::None
        )
    );

    let normal_vectors = match tangents {
        FStaticMeshVertexDataTangent::High(_data) => {
            println!("not supported normal precision");
            Vec::new()
        },
        FStaticMeshVertexDataTangent::Low(data) => {
            data.iter().map(|v| v.get_normal().get_vector()).collect()
        },
    };
    let normal_buffer_view = mesh_data.add_buffer_view({
        let startpos = buffer.len();
        let length = write_verts_buffer43(&normal_vectors, &mut buffer)?;
        GLTFBufferView::new(startpos as u32, length)
    });
    let normal_accessor = mesh_data.add_accessor(
        GLTFAccessor::new(
            normal_buffer_view,
            GLTFComponentType::Float, normal_vectors.len() as u32,
            "VEC3",
            GLTFAccessorValue::None, GLTFAccessorValue::None
        )
    );

    let uvs = lod.get_static_buffer().get_texcoords();
    let uvs: Vec<FVector2D> = match uvs {
        FStaticMeshVertexDataUV::High(data) => {
            data.into_iter().map(|v| v.get_val().clone()).collect()
        },
        FStaticMeshVertexDataUV::Low(data) => {
            data.iter().map(|v| v.get_val().get_vector()).collect()
        },
    };
    let uv_buffer_view = mesh_data.add_buffer_view({
        let startpos = buffer.len();
        let length = write_vert2_buffer(&uvs, &mut buffer)?;
        GLTFBufferView::new(startpos as u32, length)
    });
    let uv_accessor = mesh_data.add_accessor(
        GLTFAccessor::new(
            uv_buffer_view,
            GLTFComponentType::Float, uvs.len() as u32,
            "VEC2",
            GLTFAccessorValue::None, GLTFAccessorValue::None
        )
    );

    let mesh_primitive = GLTFPrimitive::new(index_accessor, material)
        .add_attribute("POSITION", mesh_accessor)
        .add_attribute("TANGENT", tangent_accessor)
        .add_attribute("NORMAL", normal_accessor)
        .add_attribute("TEXCOORD_0", uv_accessor);
    let mesh_obj = GLTFMesh::new(vec![mesh_primitive]);
    let mesh_obj = mesh_data.add_mesh(mesh_obj);

    let mesh_node = GLTFNode::new().set_mesh(mesh_obj);
    mesh_data.add_node(mesh_node);

    
    let buffer_desc = GLTFBuffer::new(buffer.len() as u32, asset_name.clone() + ".bin");
    mesh_data.add_buffer(buffer_desc);

    Ok(GLTFContainer {
        buffer,
        data: mesh_data,
    })
}

fn load_material(mesh_data: &mut GLTFItem, material_name: &str) -> GLTFMaterial {
    let material_package = Package::from_file(&("materials/".to_owned() + material_name)).unwrap();
    let material_export = material_package.get_export_move(0).unwrap();
    let material_export = match material_export.downcast::<UObject>() {
        Ok(export) => export,
        Err(_) => panic!("not a UObject"),
    };
    let material_export = *material_export;
    let texture_vals: Vec<&FPropertyTag> = material_export.get_properties().iter().filter(|v| v.get_name() == "TextureParameterValues").collect();
    let texture_vals = texture_vals[0].get_data();
    let texture_vals = match texture_vals {
        FPropertyTagType::ArrayProperty(data) => data,
        _ => panic!("not an array"),
    };

    let textures: Vec<(String, String)> = texture_vals.get_data().iter().map(|v| {
        let val_struct = match v {
            FPropertyTagType::StructProperty(data) => data.get_contents(),
            _ => panic!("not a struct"),
        };

        let texture_name = val_struct.iter().fold(None, |acc, x| {
            if x.get_name() == "ParameterValue" {
                return match x.get_data() {
                    FPropertyTagType::ObjectProperty(index) => Some(index.get_import()),
                    _ => panic!("Not an FPackageIndex"),
                };
            }
            acc
        }).unwrap();

        let texture_type = val_struct.iter().fold(None, |acc, x| {
            if x.get_name() == "ParameterInfo" {
                return match x.get_data() {
                    FPropertyTagType::StructProperty(val_props) => {
                        val_props.get_contents().iter().fold(None, |acc, y| {
                            if y.get_name() == "Name" {
                                return match y.get_data() {
                                    FPropertyTagType::NameProperty(name) => Some(name),
                                    _ => panic!("Not a name"),
                                }
                            }
                            acc
                        })
                    },
                    _ => panic!("Not a struct"),
                }
            }
            acc
        }).unwrap();
        (texture_type.to_owned(), texture_name.to_owned())
    }).collect();

    let diffuse_uri = textures.iter().fold(None, |acc, v| {
        if &v.0 == "Diffuse" {
            return Some(&v.1);
        }
        acc
    }).unwrap();

    let normal_uri = textures.iter().fold(None, |acc, v| {
        if &v.0 == "Normals" {
            return Some(&v.1);
        }
        acc
    }).unwrap();

    let diffuse_image = mesh_data.add_image(GLTFImage::new("textures/".to_owned() + diffuse_uri + ".png"));
    let normal_image = mesh_data.add_image(GLTFImage::new("textures/".to_owned() + normal_uri + ".png"));
    let default_sampler = mesh_data.add_sampler(GLTFSampler::new());

    let diffuse_texture = mesh_data.add_texture(GLTFTexture::new(diffuse_image, default_sampler.clone()));
    let normal_texture = mesh_data.add_texture(GLTFTexture::new(normal_image, default_sampler.clone()));

    GLTFMaterial::new(diffuse_texture, normal_texture)
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

    Ok(GLTFAccessorValue::Vec3Float(vec.0, vec.2, vec.1))
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

    Ok(GLTFAccessorValue::Vec3Float(vec.0, vec.2, vec.1))
}

fn write_verts_buffer(verts: &Vec<FVector>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    for i in 0..verts.len() {
        let vert = verts[i].get_tuple();
        // Unreal is left-handed, GLTF is right handed..... or the other way around
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.2)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
    }

    Ok(verts.len() as u32 * 3 * 4)
}

fn write_vert2_buffer(verts: &Vec<FVector2D>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    for i in 0..verts.len() {
        let vert = verts[i].get_tuple();
        // Unreal is left-handed, GLTF is right handed..... or the other way around
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
    }

    Ok(verts.len() as u32 * 2 * 4)
}

fn write_verts_buffer4(verts: &Vec<FVector4>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    for i in 0..verts.len() {
        let vert = verts[i].get_normal().get_tuple();
        // Unreal is left-handed, GLTF is right handed..... or the other way around
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.2)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
        cursor.write_f32::<LittleEndian>(vert.3)?;
    }

    Ok(verts.len() as u32 * 4 * 4)
}

fn write_verts_buffer43(verts: &Vec<FVector4>, buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    for i in 0..verts.len() {
        let vert = verts[i].get_normal().get_tuple3();
        // Unreal is left-handed, GLTF is right handed..... or the other way around
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.2)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
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