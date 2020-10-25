use std::io::{Cursor, Seek, SeekFrom};
use std::cell::RefCell;
use std::rc::Rc;
use byteorder::{LittleEndian, WriteBytesExt};
use std::path::Path;
use crate::assets::*;
use crate::dispatch::FNameMap;
pub mod gltf;
use gltf::*;

pub fn decode_mesh(package: Package, path: &str) -> ParserResult<GLTFContainer> {
    let filepath = Path::new(path);
    let filename = filepath.file_name().unwrap().to_str().unwrap().to_owned();

    let exports = package.get_exports();
    for export in exports {
        if let Ok(mesh) = export.downcast::<USkeletalMesh>() {
            return decode_skeletal_mesh(*mesh, filename);
        }
    }

    Err(ParserError::new(format!("Package not supported")))
}

fn decode_skeletal_mesh(mesh: USkeletalMesh, asset_name: String) -> ParserResult<GLTFContainer> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut mesh_data = GLTFItem::new();
    let materials = mesh.get_materials().clone();
    let lod = mesh.get_first_lod();

    let mut primitives = Vec::new();
    for section in lod.get_sections() {
        primitives.push(decode_skeletal_mesh_section(&mut mesh_data, &mut buffer, &lod, section, &materials)?);
    }

    let mesh_obj = mesh_data.add_mesh(GLTFMesh::new(primitives));
    let mesh_node = mesh_data.add_node(GLTFNode::new().set_mesh(mesh_obj));

    setup_skeleton(&mut mesh_data, mesh.get_skeleton(), mesh_node.clone(), &mut buffer);

    let buffer_desc = GLTFBuffer::new(buffer.len() as u32, asset_name.clone() + ".bin");
    mesh_data.add_buffer(buffer_desc);

    Ok(GLTFContainer {
        buffer,
        data: mesh_data,
    })
}

fn decode_skeletal_mesh_section(mesh_data: &mut GLTFItem, buffer: &mut Vec<u8>, lod: &FSkeletalMeshRenderData,
        section: &FSkelMeshRenderSection, materials: &Vec<FSkeletalMaterial>) -> ParserResult<GLTFPrimitive> {
    let material = materials[section.get_material_index() as usize].get_interface();
    let material = load_material(mesh_data, material)?;
    let material = mesh_data.add_material(material);
    
    let start_pos = buffer.len() as u32;
    let start_verts = section.get_base_index() as usize;
    let end_verts = section.get_num_verts() as usize + start_verts;
    let position_verts = &lod.get_position_buffer().get_verts()[start_verts..end_verts];
    let position_size = write_verts_buffer(position_verts, buffer)?;
    let position_buffer_view = GLTFBufferView::new(start_pos, position_size);
    let buffer_view = mesh_data.add_buffer_view(position_buffer_view);

    let vert_minimum = get_vert_minimum(position_verts)?;
    let vert_maximum = get_vert_maximum(position_verts)?;
    let mesh_accessor = GLTFAccessor::new(buffer_view, GLTFComponentType::Float, position_verts.len() as u32, "VEC3", vert_minimum, vert_maximum);
    let mesh_accessor = mesh_data.add_accessor(mesh_accessor);

    let indices = lod.get_indices();
    let start_indices = section.get_base_triangle_index() as usize;
    let end_indices = (section.get_num_triangles() * 3) as usize + start_indices;
    let index_view = match indices {
        FMultisizeIndexContainer::Indices16(data) => {
            let startpos = buffer.len();
            let length = write_u16_buffer(&data[start_indices..end_indices], buffer, section.get_base_index() as u32)?;
            align_writer(buffer)?;
            GLTFBufferView::new(startpos as u32, length)
        },
        FMultisizeIndexContainer::Indices32(data) => {
            let startpos = buffer.len();
            let length = write_u32_buffer(&data[start_indices..end_indices], buffer, section.get_base_index() as u32)?;
            align_writer(buffer)?;
            GLTFBufferView::new(startpos as u32, length)
        },
    };
    let index_view = mesh_data.add_buffer_view(index_view);

    let index_accessor = match indices {
        FMultisizeIndexContainer::Indices16(_data) => {
            GLTFAccessor::new(index_view, GLTFComponentType::UnsignedShort, (section.get_num_triangles() * 3) as u32, "SCALAR", GLTFAccessorValue::None, GLTFAccessorValue::None)
        },
        FMultisizeIndexContainer::Indices32(_data) => {
            GLTFAccessor::new(index_view, GLTFComponentType::UnsignedInt, (section.get_num_triangles() * 3) as u32, "SCALAR", GLTFAccessorValue::None, GLTFAccessorValue::None)
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
            data[start_verts..end_verts].iter().map(|v| v.get_tangent().get_vector()).collect()
        },
    };
    let tangent_buffer_view = {
        let startpos = buffer.len();
        let length = write_verts_buffer4(&tangent_vectors, buffer)?;
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
            data[start_verts..end_verts].iter().map(|v| v.get_normal().get_vector()).collect()
        },
    };
    let normal_buffer_view = mesh_data.add_buffer_view({
        let startpos = buffer.len();
        let length = write_verts_buffer43(&normal_vectors, buffer)?;
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
            data[start_verts..end_verts].into_iter().map(|v| v.get_val().clone()).collect()
        },
        FStaticMeshVertexDataUV::Low(data) => {
            data[start_verts..end_verts].iter().map(|v| v.get_val().get_vector()).collect()
        },
    };
    let uv_buffer_view = mesh_data.add_buffer_view({
        let startpos = buffer.len();
        let length = write_vert2_buffer(&uvs, buffer)?;
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

    let weight_buffer = lod.get_weight_buffer();
    let weight_accs = make_weight_accessors(weight_buffer, buffer, mesh_data, section, 0)?;
    let weight_accs2 = make_weight_accessors(weight_buffer, buffer, mesh_data, section, 4)?;

    let mesh_primitive = GLTFPrimitive::new(index_accessor, material)
        .add_attribute("POSITION", mesh_accessor)
        .add_attribute("TANGENT", tangent_accessor)
        .add_attribute("NORMAL", normal_accessor)
        .add_attribute("TEXCOORD_0", uv_accessor)
        .add_attribute("JOINTS_0", weight_accs.0)
        .add_attribute("WEIGHTS_0", weight_accs.1)
        .add_attribute("JOINTS_1", weight_accs2.0)
        .add_attribute("WEIGHTS_1", weight_accs2.1);

    Ok(mesh_primitive)
}

fn make_weight_accessors(weights: &FSkinWeightVertexBuffer, buffer: &mut Vec<u8>, mesh_data: &mut GLTFItem, section: &FSkelMeshRenderSection, off: usize) 
    -> ParserResult<(Rc<RefCell<GLTFAccessor>>, Rc<RefCell<GLTFAccessor>>)> {
    let joint_buffer_view = mesh_data.add_buffer_view({
        let startpos = buffer.len();
        let length = write_joints_buffer(weights, buffer, section, off)?;
        GLTFBufferView::new(startpos as u32, length)
    });
    let joint_accessor = mesh_data.add_accessor(GLTFAccessor::new(
        joint_buffer_view,
        GLTFComponentType::UnsignedShort, section.get_num_verts() as u32,
        "VEC4", GLTFAccessorValue::None, GLTFAccessorValue::None
    ));

    let weight_buffer_view = mesh_data.add_buffer_view({
        let startpos = buffer.len();
        let length = write_weights_buffer(weights, buffer, section, off)?;
        GLTFBufferView::new(startpos as u32, length)
    });
    let weight_accessor = mesh_data.add_accessor(GLTFAccessor::new(
        weight_buffer_view,
        GLTFComponentType::UnsignedByte, section.get_num_verts() as u32,
        "VEC4", GLTFAccessorValue::None, GLTFAccessorValue::None
    ).set_normalized(true));

    Ok((joint_accessor, weight_accessor))
}

fn write_joints_buffer(weights: &FSkinWeightVertexBuffer, buffer: &mut Vec<u8>, section: &FSkelMeshRenderSection, off: usize) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    let weights = weights.get_weights();

    let bone_map = section.get_bone_map();
    for i in 0..section.get_num_verts() {
        let weight = &weights[i as usize + section.get_base_index() as usize];
        let index = weight.get_bone_index();
        cursor.write_u16::<LittleEndian>(bone_map[index[0 + off] as usize])?;
        cursor.write_u16::<LittleEndian>(bone_map[index[1 + off] as usize])?;
        cursor.write_u16::<LittleEndian>(bone_map[index[2 + off] as usize])?;
        cursor.write_u16::<LittleEndian>(bone_map[index[3 + off] as usize])?;
    }

    Ok(section.get_num_verts() as u32 * 4 * 2)
}

fn write_weights_buffer(weights: &FSkinWeightVertexBuffer, buffer: &mut Vec<u8>, section: &FSkelMeshRenderSection, off: usize) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    let weights = weights.get_weights();
    for i in 0..section.get_num_verts() {
        let weight = &weights[i as usize + section.get_base_index() as usize];
        let index = weight.get_bone_weight();
        cursor.write_u8(index[0 + off])?;
        cursor.write_u8(index[1 + off])?;
        cursor.write_u8(index[2 + off])?;
        cursor.write_u8(index[3 + off])?;
    }

    Ok(section.get_num_verts() as u32 * 4)
}

pub fn transform_translation_tuple(val: (f32, f32, f32)) -> (f32, f32, f32) {
    (val.0 * 0.01, val.2 * 0.01, val.1 * 0.01)
}

pub fn transform_rotation_tuple(val: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (val.0, val.2, val.1, val.3 * -1.0)
}

fn get_translation_matrix(translation: (f32, f32, f32)) -> glm::Mat4 {
    glm::mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        translation.0, translation.1, translation.2, 1.0
    )
}

fn normalize_quat(q: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let length = (q.0*q.0 + q.1*q.1 + q.2*q.2 + q.3*q.3).sqrt();
    let n = 1.0 / length;
    (n * q.0, n * q.1, n * q.2, n * q.3)
}

fn get_rotation_matrix(qun: (f32, f32, f32, f32)) -> glm::Mat4 {
    let q = normalize_quat(qun);
    let qx = q.0;
    let qy = q.1;
    let qz = q.2;
    let qw = q.3;
    let mat = glm::mat4(
        1.0 - 2.0*qy*qy - 2.0*qz*qz, 2.0*qx*qy - 2.0*qz*qw, 2.0*qx*qz + 2.0*qy*qw, 0.0,
        2.0*qx*qy + 2.0*qz*qw, 1.0 - 2.0*qx*qx - 2.0*qz*qz, 2.0*qy*qz - 2.0*qx*qw, 0.0,
        2.0*qx*qz - 2.0*qy*qw, 2.0*qy*qz + 2.0*qx*qw, 1.0 - 2.0*qx*qx - 2.0*qy*qy, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    glm::builtin::transpose(&mat)
}

fn calculate_bind_matrix(node_index: i32, bone_list: &Vec<FMeshBoneInfo>, bone_nodes: &Vec<Rc<RefCell<GLTFNode>>>) -> glm::Mat4 {
    let mut transforms = Vec::new();
    
    let mut active_index = node_index;
    while active_index != -1 {
        let node = bone_nodes[active_index as usize].borrow();
        transforms.push((node.get_translation(), node.get_rotation()));
        active_index = bone_list[active_index as usize].get_parent_index();
    }

    let transform_matrix = 
        glm::mat4(  1.0, 0.0, 0.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                    0.0, 0.0, 1.0, 0.0,
                    0.0, 0.0, 0.0, 1.0);

    let final_mat = transforms.iter().rev().fold(transform_matrix, |acc, transform| {
        let translate = get_translation_matrix(transform.0);
        let rotate = get_rotation_matrix(transform.1);
        acc * translate * rotate
    });
    let mut inverse = glm::builtin::inverse(&final_mat);
    // I think glm::inverse has a precision issue - without setting it results in
    // values like 0.999999 and 1.000001, which cause issues.
    inverse.as_array_mut()[3].as_array_mut()[3] = 1.0;
    inverse
}

fn write_glm_vec(vec: glm::Vec4, cursor: &mut Cursor<&mut Vec<u8>>) -> ParserResult<()> {
    let slice = vec.as_array();
    cursor.write_f32::<LittleEndian>(slice[0])?;
    cursor.write_f32::<LittleEndian>(slice[1])?;
    cursor.write_f32::<LittleEndian>(slice[2])?;
    cursor.write_f32::<LittleEndian>(slice[3])?;
    Ok(())
}

fn write_matrix(mat: &glm::Mat4, buffer: &mut Vec<u8>) -> ParserResult<()> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    let vecs = mat.as_array();
    write_glm_vec(vecs[0], &mut cursor)?;
    write_glm_vec(vecs[1], &mut cursor)?;
    write_glm_vec(vecs[2], &mut cursor)?;
    write_glm_vec(vecs[3], &mut cursor)?;
    
    Ok(())
}

fn setup_skeleton(mesh_data: &mut GLTFItem, skeleton: &FReferenceSkeleton, root_node: Rc<RefCell<GLTFNode>>, buffer: &mut Vec<u8>) {  
    let bone_info = skeleton.get_bone_info();
    let bone_pose = skeleton.get_bone_pose();

    assert_eq!(bone_info.len(), bone_pose.len());

    let bone_nodes: Vec<Rc<RefCell<GLTFNode>>> = bone_pose.iter().map(|v| {
        mesh_data.add_node(GLTFNode::new().set_position(
            transform_translation_tuple(v.get_translation().get_tuple()), 
            transform_rotation_tuple(v.get_rotation().get_tuple())))
    }).collect();
    for i in 0..bone_info.len() {
        {
            let mut node = bone_nodes[i].borrow_mut();
            node.set_name(bone_info[i].get_name().to_owned());
        }
        let node = &bone_info[i];
        if node.get_parent_index() != -1 {
            bone_nodes[node.get_parent_index() as usize].borrow_mut().add_child(bone_nodes[i].clone());
        }
    }

    let startpos = buffer.len();
    for i in 0..bone_info.len() {
        let bind_matrix = calculate_bind_matrix(i as i32, &bone_info, &bone_nodes);
        write_matrix(&bind_matrix, buffer).unwrap();
    }

    let length = bone_info.len() * 16 * 4;
    let matrix_view = mesh_data.add_buffer_view(GLTFBufferView::new(startpos as u32, length as u32));
    let matrix_accessor = mesh_data.add_accessor(GLTFAccessor::new(
        matrix_view, GLTFComponentType::Float, bone_info.len() as u32,
        "MAT4", GLTFAccessorValue::None, GLTFAccessorValue::None 
    ));

    let skeleton_root = bone_nodes[0].clone();
    // Order in the joints array needs to exactly match the bone indices in the original data
    // so that the joints and weights buffers are correct
    let skin = mesh_data.add_skin(GLTFSkin::new(bone_nodes[0].clone(), bone_nodes).set_accessor(matrix_accessor));
    let mut root_node = root_node.borrow_mut();
    root_node.add_child(skeleton_root);
    root_node.set_skin(skin);
}

fn load_material(mesh_data: &mut GLTFItem, material_name: &str) -> ParserResult<GLTFMaterial> {
    let name_map = FNameMap::empty();
    let material_package = Package::from_file(&("materials/".to_owned() + material_name), &name_map)?;
    let material_export = material_package.get_export_move(0)?;
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
                    FPropertyTagType::ObjectProperty(index) => Some(match index.get_import() {
                        Some(data) => data.get_name(),
                        None => panic!("Import does not exist"),
                    }),
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

    Ok(GLTFMaterial::new(diffuse_texture, normal_texture))
}

fn get_vert_maximum(verts: &[FVector]) -> ParserResult<GLTFAccessorValue> {
    let mut vec = transform_translation_tuple(verts.get(0).unwrap().get_tuple());

    for vert in verts {
        let comp = transform_translation_tuple(vert.get_tuple());
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

fn get_vert_minimum(verts: &[FVector]) -> ParserResult<GLTFAccessorValue> {
    let mut vec = transform_translation_tuple(verts.get(0).unwrap().get_tuple());

    for vert in verts {
        let comp = transform_translation_tuple(vert.get_tuple());
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

fn write_verts_buffer(verts: &[FVector], buffer: &mut Vec<u8>) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();
    for i in 0..verts.len() {
        let vert = transform_translation_tuple(verts[i].get_tuple());
        // Unreal is left-handed, GLTF is right handed..... or the other way around
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
        cursor.write_f32::<LittleEndian>(vert.2)?;
    }

    Ok(verts.len() as u32 * 3 * 4)
}

fn align_writer(buffer: &mut Vec<u8>) -> ParserResult<()>{
    let mut writer = Cursor::new(buffer);
    writer.seek(SeekFrom::End(0)).unwrap();
    let offset_pos = (writer.position() % 4) as i64;
    for _i in 0..offset_pos {
        writer.write_u8(0)?;
    }
    Ok(())
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

fn write_u16_buffer(data: &[u16], buffer: &mut Vec<u8>, offset: u32) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();

    for item in data {
        cursor.write_u16::<LittleEndian>(((*item as u32) - offset) as u16)?;
    }

    Ok(data.len() as u32 * 2)
}

fn write_u32_buffer(data: &[u32], buffer: &mut Vec<u8>, offset: u32) -> ParserResult<u32> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();

    for item in data {
        cursor.write_u32::<LittleEndian>(*item - offset)?;
    }

    Ok(data.len() as u32 * 4)
}