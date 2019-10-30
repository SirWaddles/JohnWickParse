use byteorder::{LittleEndian, ReadBytesExt};
use super::meshes::{FReferenceSkeleton, FReferencePose};
use super::*;

#[derive(Debug, Serialize)]
enum AnimationCompressionFormat {
	None,
	Float96NoW,
	Fixed48NoW,
	IntervalFixed32NoW,
	Fixed32NoW,
	Float32NoW,
	Identity,
}

#[derive(Debug, Serialize)]
struct FAnimKeyHeader {
    key_format: AnimationCompressionFormat,
    component_mask: u32,
    num_keys: u32,
    has_time_tracks: bool,
}

impl Newable for FAnimKeyHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let packed = reader.read_u32::<LittleEndian>()?;
        let key_format_i = packed >> 28;
        let component_mask = (packed >> 24) & 0xF;
        let num_keys = packed & 0xFFFFFF;
        let key_format = match key_format_i {
            0 => AnimationCompressionFormat::None,
            1 => AnimationCompressionFormat::Float96NoW,
            2 => AnimationCompressionFormat::Fixed48NoW,
            3 => AnimationCompressionFormat::IntervalFixed32NoW,
            4 => AnimationCompressionFormat::Fixed32NoW,
            5 => AnimationCompressionFormat::Float32NoW,
            6 => AnimationCompressionFormat::Identity,
            _ => return Err(ParserError::new(format!("Unsupported format: {} {} {}: {}", key_format_i, component_mask, num_keys, packed))),
        };
        
        Ok(Self {
            key_format,
            component_mask,
            num_keys,
            has_time_tracks: (component_mask & 8) != 0,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FTrack {
    translation: Vec<FVector>,
    rotation: Vec<FQuat>,
    scale: Vec<FVector>,
    translation_times: Option<Vec<f32>>,
    rotation_times: Option<Vec<f32>>,
    scale_times: Option<Vec<f32>>,
}

impl FTrack {
    fn build_times(num_frames: i32) -> Vec<f32> {
        let mut times = Vec::new();
        for i in 0..num_frames {
            times.push(i as f32);
        }
        times
    }

    pub fn get_translation_times(&self, num_frames: i32) -> Option<Vec<f32>> {
        if self.translation.len() <= 0 {
            return None;
        }
        if self.translation.len() == 1 {
            return Some(vec![0.0]);
        }
        match &self.translation_times {
            Some(times) => Some(times.clone()),
            None => Some(Self::build_times(num_frames)),
        }
    }

    pub fn get_translation(&self) -> &Vec<FVector> {
        &self.translation
    }

    pub fn get_rotation(&self) -> &Vec<FQuat> {
        &self.rotation
    }

    pub fn get_rotation_times(&self, num_frames: i32) -> Option<Vec<f32>> {
        if self.rotation.len() <= 0 {
            return None;
        }
        if self.rotation.len() == 1 {
            return Some(vec![0.0]);
        }
        match &self.rotation_times {
            Some(times) => Some(times.clone()),
            None => Some(Self::build_times(num_frames)),
        }
    }
}

// I've based a lot of the AnimSequence stuff on the UModel implementation (thanks gildor)
// Mostly because the compression is very confusing, but also, the unreal version I have
// seems to have compression data as an array of FSmartNames.
#[derive(Debug, Serialize)]
pub struct UAnimSequence {
    super_object: UObject,
    skeleton_guid: FGuid,
    key_encoding_format: u8,
    translation_compression_format: u8,
    rotation_compression_format: u8,
    scale_compression_format: u8,
    compressed_track_offsets: Vec<i32>,
    compressed_scale_offsets: FCompressedOffsetData,
    compressed_segments: Vec<FCompressedSegment>,
    compressed_track_to_skeleton_table: Vec<i32>,
    compressed_curve_names: Vec<FSmartName>,
    compressed_raw_data_size: i32,
    compressed_num_frames: i32,
    #[serde(skip_serializing)]
    compressed_stream: Vec<u8>,
    tracks: Option<Vec<FTrack>>,
}

impl PackageExport for UAnimSequence {
    fn get_export_type(&self) -> &str {
        "AnimSequence"
    }
}

fn align_reader(reader: &mut ReaderCursor) -> ParserResult<()>{
    let offset_pos = (reader.position() % 4) as i64;
    if offset_pos == 0 { return Ok(()); }
    reader.seek(SeekFrom::Current(4 - offset_pos))?;
    Ok(())
}

fn decode_fixed48(val: u16) -> f32 {
    (val as f32) - 255.0
}

fn decode_fixed48_q(val: u16) -> f32 {
    (((val as i32) - 32767) as f32) / 32767.0
}

const Y_MASK: u32 = 0x001ffc00;
const X_MASK: u32 = 0x000003ff;

fn decode_fixed32_vec(val: u32, min: &FVector, max: &FVector) -> FVector {
    let z = val >> 21;
    let y = (val & Y_MASK) >> 10;
    let x = val & X_MASK;
    let fx = ((((x as i32) - 511) as f32) / 511.0) * max.x + min.x;
    let fy = ((((y as i32) - 1023) as f32) / 1023.0) * max.y + min.y;
    let fz = ((((z as i32) - 1023) as f32) / 1023.0) * max.z + min.z;
    FVector {
        x: fx,
        y: fy,
        z: fz,
    }
}

fn decode_fixed32_quat(val: u32, min: &FVector, max: &FVector) -> FQuat {
    let x = val >> 21;
    let y = (val & Y_MASK) >> 10;
    let z = val & X_MASK; // ignore the mismatch, it's still correct
    let fx = ((((x as i32) - 1023) as f32) / 1023.0) * max.x + min.x;
    let fy = ((((y as i32) - 1023) as f32) / 1023.0) * max.y + min.y;
    let fz = ((((z as i32) - 511) as f32) / 511.0) * max.z + min.z;
    let mut rquat = FQuat::new_raw(fx, fy, fz, 1.0);
    rquat.rebuild_w();
    rquat
}

fn read_times(reader: &mut ReaderCursor, num_keys: u32, num_frames: u32) -> ParserResult<Vec<f32>> {
    if num_keys <= 1 { return Ok(Vec::new()); }
    align_reader(reader)?;
    let mut times: Vec<f32> = Vec::new();

    if num_frames < 256 {
        for _i in 0..num_keys {
            times.push((reader.read_u8()?) as f32);
        }
    } else {
        for _i in 0..num_keys {
            times.push((reader.read_u16::<LittleEndian>()?) as f32);
        }
    }
    
    Ok(times)
}

impl UAnimSequence {
    pub(super) fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "AnimSequence")?;
        let skeleton_guid = FGuid::new(reader)?;
        let _flags = FStripDataFlags::new(reader)?;
        let use_compressed_data = reader.read_u32::<LittleEndian>()? != 0;
        if !use_compressed_data {
            return Err(ParserError::new(format!("Could not decode AnimSequence")));
        }
        let key_encoding_format = reader.read_u8()?;
        let translation_compression_format = reader.read_u8()?;
        let rotation_compression_format = reader.read_u8()?;
        let scale_compression_format = reader.read_u8()?;
        let compressed_track_offsets = read_tarray(reader)?;
        let compressed_scale_offsets = FCompressedOffsetData {
            offset_data: read_tarray(reader)?,
            strip_size: reader.read_i32::<LittleEndian>()?,
        };

        let compressed_segments = read_tarray(reader)?;
        let compressed_track_to_skeleton_table = read_tarray(reader)?;
        let compressed_curve_names = read_tarray_n(reader, name_map, import_map)?;

        let compressed_raw_data_size = reader.read_i32::<LittleEndian>()?;
        let compressed_num_frames = reader.read_i32::<LittleEndian>()?;
        let num_bytes = reader.read_i32::<LittleEndian>()?;
        let use_bulk_data_load = reader.read_u32::<LittleEndian>()? != 0;
        if use_bulk_data_load {
            panic!("Does not support BulkData for Animations");
        }
        let mut compressed_stream = vec![0u8;num_bytes as usize];
        reader.read_exact(&mut compressed_stream)?;

        /*let _curve_codec_path = read_string(reader)?;
        let _num_curve_bytes = reader.read_i32::<LittleEndian>()?;*/
        let _use_raw_data_only = reader.read_u32::<LittleEndian>()? != 0;

        let mut result = Self {
            super_object, skeleton_guid,
            key_encoding_format, translation_compression_format, rotation_compression_format, scale_compression_format,
            compressed_track_offsets, compressed_scale_offsets, compressed_segments,
            compressed_track_to_skeleton_table, compressed_curve_names, compressed_raw_data_size, compressed_num_frames,
            compressed_stream,
            tracks: None,
        };

        let tracks = match result.read_tracks() {
            Ok(data) => data,
            Err(err) => {
                println!("Error reading compressed track data: {:#?}", err);
                return Ok(result);
            },
        };
        result.tracks = Some(tracks);
        Ok(result)
    }

    pub fn get_super_object(&self) -> &UObject {
        &self.super_object
    }

    pub fn get_track_map(&self) -> Vec<i32> {
        // use UObject later
        self.compressed_track_to_skeleton_table.clone()
    }

    pub fn get_num_frames(&self) -> i32 {
        self.compressed_num_frames
    }

    pub fn get_tracks(self) -> Vec<FTrack> {
        self.tracks.unwrap()
    }

    pub fn find_track(&self, track_id: i32) -> Option<usize> {
        let track_map = &self.compressed_track_to_skeleton_table;
        for i in 0..track_map.len() {
            if track_id == track_map[i] {
                return Some(i);
            }
        }
        None
    }

    pub fn add_tracks(&mut self, to_add: UAnimSequence) {
        let track_map = to_add.get_track_map();
        let tracks = to_add.get_tracks();

        for (i, track_id) in track_map.into_iter().enumerate() {
            let self_track_id = self.find_track(track_id);
            if let None = self_track_id { continue; }
            let track = match &mut self.tracks {
                Some(self_tracks) => self_tracks,
                None => return,
            }.get_mut(self_track_id.unwrap() as usize).unwrap();
            let track_add = &tracks[i];
            for (e, translate) in track_add.translation.iter().enumerate() {
                match track.translation.get_mut(e) {
                    Some(translate_self) => {
                        translate_self.x += translate.x;
                        translate_self.y += translate.y;
                        translate_self.z += translate.z;
                    },
                    None => continue,
                }
            }
            for (e, rotate) in track_add.rotation.iter().enumerate() {
                match track.rotation.get_mut(e) {
                    Some(rotate_self) => {
                        rotate_self.x += rotate.x;
                        rotate_self.y += rotate.y;
                        rotate_self.z += rotate.z;
                        rotate_self.normalize();
                    },
                    None => continue,
                }
            }
        }
    }

    fn read_tracks(&self) -> ParserResult<Vec<FTrack>> {
        if self.key_encoding_format != 2 {
            return Err(ParserError::new(format!("Can only parse PerTrackCompression")));
        }
        let mut reader = ReaderCursor::new(&self.compressed_stream);
        let num_tracks = self.compressed_track_offsets.len() / 2;
        // TODO: Use UObject property instead.
        let num_frames = self.compressed_num_frames;

        let mut tracks = Vec::new();

        for track_i in 0..num_tracks {
            let mut translates: Vec<FVector> = Vec::new();
            let mut rotates: Vec<FQuat> = Vec::new();
            let mut scales: Vec<FVector> = Vec::new();
            let mut translation_times = None;
            let mut rotation_times = None;
            let mut scale_times = None;
            { // Translation
                let offset = self.compressed_track_offsets[track_i * 2];
                if offset != -1 {
                    let header = FAnimKeyHeader::new(&mut reader).map_err(|v| ParserError::add(v, format!("Translation error: {} {}", reader.position(), track_i)))?;
                    let mut min = FVector::unit();
                    let mut max = FVector::unit();

                    if let AnimationCompressionFormat::IntervalFixed32NoW = header.key_format {
                        if header.component_mask & 1 != 0 {
                            min.x = reader.read_f32::<LittleEndian>()?;
                            max.x = reader.read_f32::<LittleEndian>()?;
                        }
                        if header.component_mask & 2 != 0 {
                            min.y = reader.read_f32::<LittleEndian>()?;
                            max.y = reader.read_f32::<LittleEndian>()?;
                        }
                        if header.component_mask & 4 != 0 {
                            min.z = reader.read_f32::<LittleEndian>()?;
                            max.z = reader.read_f32::<LittleEndian>()?;
                        }
                    }

                    for _key in 0..header.num_keys {
                        let translate = match header.key_format {
                            AnimationCompressionFormat::None | AnimationCompressionFormat::Float96NoW => {
                                let mut fvec = FVector::unit();
                                if header.component_mask & 7 != 0 {
                                    if header.component_mask & 1 != 0 { fvec.x = reader.read_f32::<LittleEndian>()?; }
                                    if header.component_mask & 2 != 0 { fvec.y = reader.read_f32::<LittleEndian>()?; }
                                    if header.component_mask & 4 != 0 { fvec.z = reader.read_f32::<LittleEndian>()?; }
                                } else {
                                    fvec = FVector::new(&mut reader)?;
                                }
                                fvec
                            },
                            AnimationCompressionFormat::Fixed48NoW => {
                                let mut fvec = FVector::unit();
                                if header.component_mask & 1 != 0 { fvec.x = decode_fixed48(reader.read_u16::<LittleEndian>()?); }
                                if header.component_mask & 2 != 0 { fvec.y = decode_fixed48(reader.read_u16::<LittleEndian>()?); }
                                if header.component_mask & 4 != 0 { fvec.z = decode_fixed48(reader.read_u16::<LittleEndian>()?); }
                                fvec
                            },
                            AnimationCompressionFormat::IntervalFixed32NoW => {
                                let val = reader.read_u32::<LittleEndian>()?;
                                decode_fixed32_vec(val, &min, &max)
                            },
                            _ => panic!("key format: {:#?}", header.key_format),
                        };

                        translates.push(translate);
                    }

                    if header.has_time_tracks {
                        translation_times = Some(read_times(&mut reader, header.num_keys, num_frames as u32)?);
                    }
                    align_reader(&mut reader)?;
                    
                    //println!("anim track: {} 0 {}", track_i, reader.position());
                }
            }

            { // Rotation
                let offset = self.compressed_track_offsets[(track_i * 2) + 1];
                if offset != -1 {
                    let header = FAnimKeyHeader::new(&mut reader).map_err(|v| ParserError::add(v, format!("Rotation error: {} {}", reader.position(), track_i)))?;
                    let mut min = FVector::unit();
                    let mut max = FVector::unit();

                    if let AnimationCompressionFormat::IntervalFixed32NoW = header.key_format {
                        if header.component_mask & 1 != 0 {
                            min.x = reader.read_f32::<LittleEndian>()?;
                            max.x = reader.read_f32::<LittleEndian>()?;
                        }
                        if header.component_mask & 2 != 0 {
                            min.y = reader.read_f32::<LittleEndian>()?;
                            max.y = reader.read_f32::<LittleEndian>()?;
                        }
                        if header.component_mask & 4 != 0 {
                            min.z = reader.read_f32::<LittleEndian>()?;
                            max.z = reader.read_f32::<LittleEndian>()?;
                        }
                    }

                    for _key in 0..header.num_keys {
                        let rotate = match header.key_format {
                            AnimationCompressionFormat::None | AnimationCompressionFormat::Float96NoW => {
                                let mut fvec = FVector::unit();
                                if header.component_mask & 7 != 0 {
                                    if header.component_mask & 1 != 0 { fvec.x = reader.read_f32::<LittleEndian>()?; }
                                    if header.component_mask & 2 != 0 { fvec.y = reader.read_f32::<LittleEndian>()?; }
                                    if header.component_mask & 4 != 0 { fvec.z = reader.read_f32::<LittleEndian>()?; }
                                } else {
                                    fvec = FVector::new(&mut reader)?;
                                }
                                let mut fquat = FQuat {
                                    x: fvec.x,
                                    y: fvec.y,
                                    z: fvec.z,
                                    w: 0.0,
                                };
                                fquat.rebuild_w();
                                fquat
                            },
                            AnimationCompressionFormat::Fixed48NoW => {
                                let mut fquat = FQuat::unit();
                                if header.component_mask & 1 != 0 { fquat.x = decode_fixed48_q(reader.read_u16::<LittleEndian>()?); }
                                if header.component_mask & 2 != 0 { fquat.y = decode_fixed48_q(reader.read_u16::<LittleEndian>()?); }
                                if header.component_mask & 4 != 0 { fquat.z = decode_fixed48_q(reader.read_u16::<LittleEndian>()?); }
                                fquat.rebuild_w();
                                fquat
                            },
                            AnimationCompressionFormat::IntervalFixed32NoW => {
                                let val = reader.read_u32::<LittleEndian>()?;
                                decode_fixed32_quat(val, &min, &max)
                            },
                            _ => panic!("key format: {:#?}", header.key_format),
                        };

                        rotates.push(rotate);
                        
                    }

                    if header.has_time_tracks {
                        rotation_times = Some(read_times(&mut reader, header.num_keys, num_frames as u32)?);
                    }
                    align_reader(&mut reader)?;
                    //println!("track info: {} {} {:?}", header.component_mask, header.num_keys, header.key_format);
                    //println!("anim track: {} 1 {}", track_i, reader.position());
                }
            }

            { // Scale
                let offset = self.compressed_scale_offsets.offset_data[track_i * self.compressed_scale_offsets.strip_size as usize];
                if offset != -1 {
                    let header = FAnimKeyHeader::new(&mut reader).map_err(|v| ParserError::add(v, format!("Scale error: {} {}", reader.position(), track_i)))?;
                    let mut min = FVector::unit();
                    let mut max = FVector::unit();

                    if let AnimationCompressionFormat::IntervalFixed32NoW = header.key_format {
                        if header.component_mask & 1 != 0 {
                            min.x = reader.read_f32::<LittleEndian>()?;
                            max.x = reader.read_f32::<LittleEndian>()?;
                        }
                        if header.component_mask & 2 != 0 {
                            min.y = reader.read_f32::<LittleEndian>()?;
                            max.y = reader.read_f32::<LittleEndian>()?;
                        }
                        if header.component_mask & 4 != 0 {
                            min.z = reader.read_f32::<LittleEndian>()?;
                            max.z = reader.read_f32::<LittleEndian>()?;
                        }
                    }

                    for _key in 0..header.num_keys {
                        let scale = match header.key_format {
                            AnimationCompressionFormat::None | AnimationCompressionFormat::Float96NoW => {
                                let mut fvec = FVector::unit_scale();
                                if header.component_mask & 7 != 0 {
                                    if header.component_mask & 1 != 0 { fvec.x = reader.read_f32::<LittleEndian>()?; }
                                    if header.component_mask & 2 != 0 { fvec.y = reader.read_f32::<LittleEndian>()?; }
                                    if header.component_mask & 4 != 0 { fvec.z = reader.read_f32::<LittleEndian>()?; }
                                } else {
                                    fvec = FVector::new(&mut reader)?;
                                }
                                fvec
                            },
                            AnimationCompressionFormat::Fixed48NoW => {
                                let mut fvec = FVector::unit_scale();
                                if header.component_mask & 1 != 0 { fvec.x = decode_fixed48(reader.read_u16::<LittleEndian>()?); }
                                if header.component_mask & 2 != 0 { fvec.y = decode_fixed48(reader.read_u16::<LittleEndian>()?); }
                                if header.component_mask & 4 != 0 { fvec.z = decode_fixed48(reader.read_u16::<LittleEndian>()?); }
                                fvec
                            },
                            AnimationCompressionFormat::IntervalFixed32NoW => {
                                let val = reader.read_u32::<LittleEndian>()?;
                                decode_fixed32_vec(val, &min, &max)
                            },
                            _ => panic!("key format: {:#?}", header.key_format),
                        };

                        scales.push(scale);
                    }

                    if header.has_time_tracks {
                        scale_times = Some(read_times(&mut reader, header.num_keys, num_frames as u32)?);
                    }
                    align_reader(&mut reader)?;
                    //println!("track info: {} {} {:?}", header.component_mask, header.num_keys, header.key_format);
                    //println!("anim track: {} 2 {}", track_i, reader.position());
                }
            }

            tracks.push(FTrack {
                translation: translates,
                rotation: rotates,
                scale: scales,
                translation_times, rotation_times, scale_times,
            });
        }

        if reader.position() != self.compressed_stream.len() as u64 {
            println!("Could not read tracks correctly, {} bytes remaining", self.compressed_stream.len() as u64 - reader.position());
        }

        Ok(tracks)
    }
}

#[derive(Debug, Serialize)]
pub struct USkeleton {
    super_object: UObject,
    reference_skeleton: FReferenceSkeleton,
    anim_retarget_sources: Vec<(String, FReferencePose)>,
}

impl PackageExport for USkeleton {
    fn get_export_type(&self) -> &str {
        "Skeleton"
    }
}

impl USkeleton {
    pub(super) fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "Skeleton")?;
        let reference_skeleton = FReferenceSkeleton::new_n(reader, name_map, import_map)?;

        let mut anim_retarget_sources = Vec::new();
        let anim_length = reader.read_u32::<LittleEndian>()?;
        for _i in 0..anim_length {
            let retarget_name = read_fname(reader, name_map)?;
            let retarget_pose = FReferencePose::new_n(reader, name_map, import_map)?;
            anim_retarget_sources.push((retarget_name, retarget_pose));
        }

        Ok(Self {
            super_object, 
            reference_skeleton,
            anim_retarget_sources,
        })
    }

    pub fn get_reference(&self) -> &FReferenceSkeleton {
        &self.reference_skeleton
    }
}