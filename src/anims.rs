use std::io::{Cursor, Seek, SeekFrom};
use byteorder::{LittleEndian, WriteBytesExt};
use std::path::Path;
use crate::meshes::*;
use crate::meshes::gltf::*;
use crate::assets::*;
use crate::dispatch::FNameMap;

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

pub fn decode_anim_type(anim: UAnimSequence, asset_name: String) -> ParserResult<GLTFContainer> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut item = GLTFItem::new();

    let skeleton_map = get_skeleton_map(&anim)?;
    let track_map = anim.get_track_map();
    let num_frames = anim.get_num_frames();
    let tracks = anim.get_tracks();

    let mut animation = GLTFAnimation::new();

    for (i, track) in tracks.into_iter().enumerate() {
        let bone_name = match skeleton_map.get(track_map[i] as usize) {
            Some(name) => name.clone(),
            None => continue,
        };
        write_track(&track, &mut buffer, &mut item, &mut animation, num_frames, bone_name)?;
    }

    let buffer_desc = GLTFBuffer::new(buffer.len() as u32, asset_name.clone() + ".bin");
    item.add_buffer(buffer_desc);

    item.add_animation(animation);

    Ok(GLTFContainer {
        buffer,
        data: item,
    })
}

fn transform_rotation_tuple2(q: (f32, f32, f32 ,f32)) -> (f32, f32, f32, f32) {
    (q.0, q.2, q.1, q.3)
}

pub fn write_verts_buffer(verts: &Vec<FVector>, cursor: &mut Cursor<&mut Vec<u8>>) -> ParserResult<u32> {
    for vector in verts {
        let vert = transform_translation_tuple(vector.get_tuple());
        cursor.write_f32::<LittleEndian>(vert.0)?;
        cursor.write_f32::<LittleEndian>(vert.1)?;
        cursor.write_f32::<LittleEndian>(vert.2)?;
    }

    Ok(verts.len() as u32 * 3 * 4)
}

pub fn write_quats_buffer(quats: &Vec<FQuat>, cursor: &mut Cursor<&mut Vec<u8>>) -> ParserResult<u32> {
    for quaternion in quats {
        let quat = transform_rotation_tuple2(quaternion.conjugate().get_tuple());
        cursor.write_f32::<LittleEndian>(quat.0)?;
        cursor.write_f32::<LittleEndian>(quat.1)?;
        cursor.write_f32::<LittleEndian>(quat.2)?;
        cursor.write_f32::<LittleEndian>(quat.3)?;
    }

    Ok(quats.len() as u32 * 4 * 4)
}

fn write_times(times: Vec<f32>, cursor: &mut Cursor<&mut Vec<u8>>) -> ParserResult<u32> {
    let len = times.len() as u32;
    for time in times {
        cursor.write_f32::<LittleEndian>(time)?;
    }
    Ok(len * 4)
}

fn write_track(track: &FTrack, buffer: &mut Vec<u8>, item: &mut GLTFItem, anim: &mut GLTFAnimation, 
        num_frames: i32, bone_name: String) -> ParserResult<()> {
    let mut cursor = Cursor::new(buffer);
    cursor.seek(SeekFrom::End(0)).unwrap();

    let t_times: Vec<f32> = match track.get_translation_times(num_frames) {
        Some(times) => times,
        None => return Ok(()), // skip track
    }.into_iter().map(|v| v / 30.0).collect();
    let t_times_len = t_times.len() as u32;
    let t_times_max: f32 = t_times.iter().fold(0.0, |acc, v| {
        match *v > acc {
            true => *v,
            false => acc,
        }
    });
    let t_times_input_view = item.add_buffer_view({
        let startpos = cursor.position();
        let length = write_times(t_times, &mut cursor)?;
        GLTFBufferView::new(startpos as u32, length)
    });
    let t_times_output_view = item.add_buffer_view({
        let startpos = cursor.position();
        let length = write_verts_buffer(track.get_translation(), &mut cursor)?;
        GLTFBufferView::new(startpos as u32, length)
    });

    let t_input_accessor = item.add_accessor(GLTFAccessor::new(
        t_times_input_view, GLTFComponentType::Float, t_times_len, "SCALAR",
        GLTFAccessorValue::ScalarFloat(0.0), GLTFAccessorValue::ScalarFloat(t_times_max)
    ));

    let t_output_accessor = item.add_accessor(GLTFAccessor::new(
        t_times_output_view, GLTFComponentType::Float, track.get_translation().len() as u32, "VEC3",
        GLTFAccessorValue::None, GLTFAccessorValue::None 
    ));

    let t_sampler = anim.add_sampler(GLTFAnimationSampler::new(
        t_input_accessor, t_output_accessor, GLTFInterpolation::Linear
    ));

    anim.add_channel(GLTFChannel::new(
        t_sampler, GLTFAnimationTarget::new(
            "translation", bone_name.clone()
        )
    ));

    // copy-pasted from above, not sure how to dry this.
    let r_times: Vec<f32> = match track.get_rotation_times(num_frames) {
        Some(times) => times,
        None => return Ok(()), // skip track
    }.into_iter().map(|v| v / 30.0).collect();
    let r_times_len = r_times.len() as u32;
    let r_times_max: f32 = r_times.iter().fold(0.0, |acc, v| {
        match *v > acc {
            true => *v,
            false => acc,
        }
    });
    let r_times_input_view = item.add_buffer_view({
        let startpos = cursor.position();
        let length = write_times(r_times, &mut cursor)?;
        GLTFBufferView::new(startpos as u32, length)
    });
    let r_times_output_view = item.add_buffer_view({
        let startpos = cursor.position();
        let length = write_quats_buffer(track.get_rotation(), &mut cursor)?;
        GLTFBufferView::new(startpos as u32, length)
    });

    let r_input_accessor = item.add_accessor(GLTFAccessor::new(
        r_times_input_view, GLTFComponentType::Float, r_times_len, "SCALAR",
        GLTFAccessorValue::ScalarFloat(0.0), GLTFAccessorValue::ScalarFloat(r_times_max)
    ));

    let r_output_accessor = item.add_accessor(GLTFAccessor::new(
        r_times_output_view, GLTFComponentType::Float, track.get_rotation().len() as u32, "VEC4",
        GLTFAccessorValue::None, GLTFAccessorValue::None 
    ));

    let r_sampler = anim.add_sampler(GLTFAnimationSampler::new(
        r_input_accessor, r_output_accessor, GLTFInterpolation::Linear
    ));

    anim.add_channel(GLTFChannel::new(
        r_sampler, GLTFAnimationTarget::new(
            "rotation", bone_name.clone()
        )
    ));

    Ok(())
}

fn get_skeleton_map(anim: &UAnimSequence) -> ParserResult<Vec<String>> {
    let property = anim.get_super_object().get_properties().iter().fold(None, |acc, v| {
        if v.get_name() == "Skeleton" {
            return Some(v);
        }
        acc
    }).unwrap();
    let name_map = FNameMap::empty();
    let path = match property.get_data() {
        FPropertyTagType::ObjectProperty(import) => import.get_import(),
        _ => return Err(ParserError::new(format!("Skeleton unreadable format"))),
    };
    let path = match path {
        Some(data) => data.get_name(),
        None => return Err(ParserError::new(format!("Import not valid"))),
    };

    let skeleton_path = "skeletons/".to_owned() + path;
    let package = Package::from_file(&skeleton_path, &name_map)?;
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