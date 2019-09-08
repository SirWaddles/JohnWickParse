use byteorder::{LittleEndian, ReadBytesExt};
use super::*;

#[derive(Debug, Serialize)]
pub struct FPositionVertexBuffer {
    verts: Vec<FVector>,
    stride: i32,
    num_verts: i32,
}

impl FPositionVertexBuffer {
    pub fn get_verts(&self) -> &[FVector] {
        &self.verts[..]
    }
}

impl Newable for FPositionVertexBuffer {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let stride = reader.read_i32::<LittleEndian>()?;
        let num_verts = reader.read_i32::<LittleEndian>()?;
        let _element_size = reader.read_i32::<LittleEndian>()?;
        let verts = read_tarray(reader)?;
        Ok(Self {
            stride, num_verts, verts,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FPackedRGBA16N {
    x: i16,
    y: i16,
    z: i16,
    w: i16,
}

impl Newable for FPackedRGBA16N {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_i16::<LittleEndian>()?,
            y: reader.read_i16::<LittleEndian>()?,
            z: reader.read_i16::<LittleEndian>()?,
            w: reader.read_i16::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FPackedNormal {
    x: i8,
    y: i8,
    z: i8,
    w: i8,
}

impl Newable for FPackedNormal {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_i8()?,
            y: reader.read_i8()?,
            z: reader.read_i8()?,
            w: reader.read_i8()?,
        })
    }
}

fn rescale_i8(val: i8) -> f32 {
    (val as f32) * (1.0f32 / 127.0f32)
}

impl FPackedNormal {
    pub fn get_vector(&self) -> FVector4 {
        FVector4 {
            x: rescale_i8(self.x),
            y: rescale_i8(self.y),
            z: rescale_i8(self.z),
            w: rescale_i8(self.w),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FVector2DHalf {
    x: f16,
    y: f16,
}

impl Newable for FVector2DHalf {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: f16::from_bits(reader.read_u16::<LittleEndian>()?),
            y: f16::from_bits(reader.read_u16::<LittleEndian>()?),
        })
    }
}

impl FVector2DHalf {
    pub fn get_vector(&self) -> FVector2D {
        FVector2D {
            x: self.x.to_f32(),
            y: self.y.to_f32(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TStaticMeshVertexTangent<T> {
    normal: T,
    tangent: T,
}

impl<T> Newable for TStaticMeshVertexTangent<T> where T: Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            tangent: T::new(reader)?,
            normal: T::new(reader)?,
        })
    }
}

impl<T> TStaticMeshVertexTangent<T> {
    pub fn get_normal(&self) -> &T {
        &self.normal
    }

    pub fn get_tangent(&self) -> &T {
        &self.tangent
    }
}

#[derive(Debug, Serialize)]
pub struct TStaticMeshVertexUV<T> {
    value: T,
}

impl<T> Newable for TStaticMeshVertexUV<T> where T: Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            value: T::new(reader)?,
        })
    }
}

impl<T> TStaticMeshVertexUV<T> {
    pub fn get_val(&self) -> &T {
        &self.value
    }
}

#[derive(Debug, Serialize)]
pub enum FStaticMeshVertexDataTangent {
    High(Vec<TStaticMeshVertexTangent<FPackedRGBA16N>>),
    Low(Vec<TStaticMeshVertexTangent<FPackedNormal>>),
}

#[derive(Debug, Serialize)]
pub enum FStaticMeshVertexDataUV {
    High(Vec<TStaticMeshVertexUV<FVector2D>>),
    Low(Vec<TStaticMeshVertexUV<FVector2DHalf>>),
}

#[derive(Debug, Serialize)]
pub struct FStaticMeshVertexBuffer {
    num_tex_coords: i32,
    num_vertices: i32,
    tangents: FStaticMeshVertexDataTangent,
    uvs: FStaticMeshVertexDataUV,
}

impl FStaticMeshVertexBuffer {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Option<Self>> {
        let flags = FStripDataFlags::new(reader)?;

        let num_tex_coords = reader.read_i32::<LittleEndian>()?;
        let num_vertices = reader.read_i32::<LittleEndian>()?;
        let use_full_precision_uvs = reader.read_i32::<LittleEndian>()? != 0;
        let use_high_precision_tangent = reader.read_i32::<LittleEndian>()? != 0;

        if flags.is_data_stripped_for_server() {
            return Ok(None);
        }

        let _element_size = reader.read_i32::<LittleEndian>()?;
        let tangents = match use_high_precision_tangent {
            true => FStaticMeshVertexDataTangent::High(read_tarray(reader)?),
            false => FStaticMeshVertexDataTangent::Low(read_tarray(reader)?),
        };

        let _element_size = reader.read_i32::<LittleEndian>()?;
        let uvs = match use_full_precision_uvs {
            true => FStaticMeshVertexDataUV::High(read_tarray(reader)?),
            false => FStaticMeshVertexDataUV::Low(read_tarray(reader)?),
        };

        Ok(Some(Self {
            num_tex_coords, num_vertices, tangents, uvs,
        }))
    }

    pub fn get_tangents(&self) -> &FStaticMeshVertexDataTangent {
        &self.tangents
    }
    
    pub fn get_texcoords(&self) -> &FStaticMeshVertexDataUV {
        &self.uvs
    }
}

#[derive(Debug, Serialize)]
pub struct FSkinWeightInfo {
    bone_index: [u8;8],
    bone_weight: [u8;8],
}

impl FSkinWeightInfo {
    fn new(reader: &mut ReaderCursor, max_influences: usize) -> ParserResult<Self> {
        if max_influences > 8 {
            return Err(ParserError::new(format!("Max influences too high")));
        }

        let mut bone_index = [0u8;8];
        for i in 0..max_influences {
            bone_index[i] = reader.read_u8()?;
        }
        let mut bone_weight = [0u8;8];
        for i in 0..max_influences {
            bone_weight[i] = reader.read_u8()?;
        }

        Ok(Self {
            bone_index, bone_weight,
        })
    }

    pub fn get_bone_index(&self) -> &[u8;8] {
        &self.bone_index
    }

    pub fn get_bone_weight(&self) -> &[u8;8] {
        &self.bone_weight
    }
}

#[derive(Debug, Serialize)]
pub struct FSkinWeightVertexBuffer {
    weights: Vec<FSkinWeightInfo>,
    num_vertices: i32,
}

impl FSkinWeightVertexBuffer {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Option<Self>> {
        let flags = FStripDataFlags::new(reader)?;

        let extra_bone_influences = reader.read_i32::<LittleEndian>()? != 0;
        let num_vertices = reader.read_i32::<LittleEndian>()?;

        if flags.is_data_stripped_for_server() {
            return Ok(None);
        }

        let _element_size = reader.read_i32::<LittleEndian>()?;
        let element_count = reader.read_i32::<LittleEndian>()?;
        let num_influences = match extra_bone_influences {
            true => 8,
            false => 4,
        };
        let mut weights = Vec::new();
        for _i in 0..element_count {
            weights.push(FSkinWeightInfo::new(reader, num_influences)?);
        }

        Ok(Some(Self {
            weights, num_vertices,
        }))
    }

    pub fn get_weights(&self) -> &Vec<FSkinWeightInfo> {
        &self.weights
    }
}

#[derive(Debug, Serialize)]
pub struct FColorVertexBuffer {
    stride: i32,
    num_verts: i32,
    colours: Vec<FColor>,
}

impl Newable for FColorVertexBuffer {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let flags = FStripDataFlags::new(reader)?;
        let stride = reader.read_i32::<LittleEndian>()?;
        let num_verts = reader.read_i32::<LittleEndian>()?;
        let colours = match !flags.is_data_stripped_for_server() && num_verts > 0 {
            true => {
                let _element_size = reader.read_i32::<LittleEndian>()?;
                read_tarray(reader)?
            },
            false => Vec::new(),
        };

        Ok(Self {
            stride, num_verts, colours,
        })
    }
}

#[derive(Debug, Serialize)]
struct FBoxSphereBounds {
    origin: FVector,
    box_extend: FVector,
    sphere_radius: f32,
}

impl Newable for FBoxSphereBounds {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            origin: FVector::new(reader)?,
            box_extend: FVector::new(reader)?,
            sphere_radius: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct FSkeletalMaterial {
    material_interface: FPackageIndex,
    material_slot_name: String,
    uv_channel_data: FMeshUVChannelInfo,
}

impl FSkeletalMaterial {
    pub fn get_interface(&self) -> &str {
        match &self.material_interface.import {
            Some(data) => &data.object_name,
            None => panic!("No import exists"),
        }
    }
}

impl NewableWithNameMap for FSkeletalMaterial {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let material_interface = FPackageIndex::new_n(reader, name_map, import_map)?;
        let serialize_slot_name = reader.read_u32::<LittleEndian>()? != 0;
        let material_slot_name = match serialize_slot_name {
            true => read_fname(reader, name_map)?,
            false => "".to_owned(),
        };
        let uv_channel_data = FMeshUVChannelInfo::new(reader)?;
        Ok(Self {
            material_interface,
            material_slot_name,
            uv_channel_data,
        })
    }
}

#[derive(Debug, Serialize, Clone)]
struct FMeshUVChannelInfo {
    initialised: bool,
    override_densities: bool,
    local_uv_densities: [f32;4],
}

impl Newable for FMeshUVChannelInfo {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let initialised = reader.read_u32::<LittleEndian>()? != 0;
        let override_densities = reader.read_u32::<LittleEndian>()? != 0;
        let mut local_uv_densities = [0.0;4];
        for i in 0..4 {
            local_uv_densities[i] = reader.read_f32::<LittleEndian>()?;
        }

        Ok(Self {
            initialised, override_densities, local_uv_densities,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FTransform {
    rotation: FQuat,
    translation: FVector,
    scale_3d: FVector,
}

#[allow(dead_code)]
impl FTransform {
    pub fn get_rotation(&self) -> &FQuat {
        &self.rotation
    }

    pub fn get_translation(&self) -> &FVector {
        &self.translation
    }

    pub fn get_scale(&self) -> &FVector {
        &self.scale_3d
    }
}

impl Newable for FTransform {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            rotation: FQuat::new(reader)?,
            translation: FVector::new(reader)?,
            scale_3d: FVector::new(reader)?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FMeshBoneInfo {
    name: String,
    parent_index: i32,
}

impl FMeshBoneInfo {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_parent_index(&self) -> i32 {
        self.parent_index
    }
}

impl NewableWithNameMap for FMeshBoneInfo {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            name: read_fname(reader, name_map)?,
            parent_index: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FReferenceSkeleton {
    ref_bone_info: Vec<FMeshBoneInfo>,
    ref_bone_pose: Vec<FTransform>,
    name_to_index: Vec<(String, i32)>,
}

impl FReferenceSkeleton {
    pub fn get_bone_info(&self) -> &Vec<FMeshBoneInfo> {
        &self.ref_bone_info
    }

    pub fn get_bone_pose(&self) -> &Vec<FTransform> {
        &self.ref_bone_pose
    }
}

impl NewableWithNameMap for FReferenceSkeleton {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let ref_bone_info = read_tarray_n(reader, name_map, import_map)?;
        let ref_bone_pose = read_tarray(reader)?;
        let index_count = reader.read_u32::<LittleEndian>()?;

        let mut name_to_index = Vec::new();
        for _i in 0..index_count {
            name_to_index.push((read_fname(reader, name_map)?, reader.read_i32::<LittleEndian>()?));
        }

        Ok(Self {
            ref_bone_info, ref_bone_pose, name_to_index,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FReferencePose {
    pose_name: String,
    reference_pose: Vec<FTransform>,
}

impl NewableWithNameMap for FReferencePose {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            pose_name: read_fname(reader, name_map)?,
            reference_pose: read_tarray(reader)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FClothingSectionData {
    asset_guid: FGuid,
    asset_lod_index: i32,
}

impl Newable for FClothingSectionData {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            asset_guid: FGuid::new(reader)?,
            asset_lod_index: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FIndexLengthPair {
    word1: u32,
    word2: u32,
}

impl Newable for FIndexLengthPair {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            word1: reader.read_u32::<LittleEndian>()?,
            word2: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub enum FMultisizeIndexContainer {
    Indices16(Vec<u16>),
    Indices32(Vec<u32>),
}

impl Newable for FMultisizeIndexContainer {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let data_size = reader.read_u8()?;
        let _element_size = reader.read_i32::<LittleEndian>()?;
        match data_size {
            2 => Ok(FMultisizeIndexContainer::Indices16(read_tarray(reader)?)),
            4 => Ok(FMultisizeIndexContainer::Indices32(read_tarray(reader)?)),
            _ => Err(ParserError::new(format!("No format size"))),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FSkelMeshRenderSection {
    material_index: i16,
    base_index: i32,
    num_triangles: i32,
    base_vertex_index: u32,
    cloth_mapping_data: Vec<FMeshToMeshVertData>,
    bone_map: Vec<u16>,
    num_vertices: i32,
    max_bone_influences: i32,
    clothing_data: FClothingSectionData,
    disabled: bool,
}

impl FSkelMeshRenderSection {
    pub fn get_bone_map(&self) -> &Vec<u16> {
        &self.bone_map
    }

    pub fn get_num_verts(&self) -> i32 {
        self.num_vertices
    }

    pub fn get_base_index(&self) -> u32 {
        self.base_vertex_index
    }

    pub fn get_num_triangles(&self) -> i32 {
        self.num_triangles
    }

    pub fn get_base_triangle_index(&self) -> i32 {
        self.base_index
    }

    pub fn get_material_index(&self) -> i16 {
        self.material_index
    }
}

impl NewableWithNameMap for FSkelMeshRenderSection {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        let flags = FStripDataFlags::new(reader)?;
        let material_index = reader.read_i16::<LittleEndian>()?;
        let base_index = reader.read_i32::<LittleEndian>()?;
        let num_triangles = reader.read_i32::<LittleEndian>()?;

        let _recompute_tangent = reader.read_u32::<LittleEndian>()? != 0;
        let _cast_shadow = reader.read_u32::<LittleEndian>()? != 0;
        let mut base_vertex_index = 0;
        if !flags.is_data_stripped_for_server() {
            base_vertex_index = reader.read_u32::<LittleEndian>()?;
        }
        let cloth_mapping_data = read_tarray(reader)?;
        let bone_map = read_tarray(reader)?;
        let num_vertices = reader.read_i32::<LittleEndian>()?;
        let max_bone_influences = reader.read_i32::<LittleEndian>()?;
        let _correspond_cloth_asset_index = reader.read_i16::<LittleEndian>()?;
        let clothing_data = FClothingSectionData::new(reader)?;
        let _vertex_buffer: Vec<i32> = read_tarray(reader)?;
        let _index_pairs: Vec<FIndexLengthPair> = read_tarray(reader)?;
        let disabled = reader.read_u32::<LittleEndian>()? != 0;

        Ok(Self {
            material_index, base_index, num_triangles, base_vertex_index, cloth_mapping_data,
            bone_map, num_vertices, max_bone_influences, clothing_data, disabled,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FSkeletalMeshRenderData {
    sections: Vec<FSkelMeshRenderSection>,
    indices: FMultisizeIndexContainer,
    active_bone_indices: Vec<i16>,
    required_bones: Vec<i16>,
    position_vertex_buffer: FPositionVertexBuffer,
    static_mesh_vertex_buffer: Option<FStaticMeshVertexBuffer>,
    skin_weight_vertex_buffer: Option<FSkinWeightVertexBuffer>,
    colour_vertex_buffer: Option<FColorVertexBuffer>,
}

impl FSkeletalMeshRenderData {
    pub fn get_position_buffer(&self) -> &FPositionVertexBuffer {
        &self.position_vertex_buffer
    }

    pub fn get_indices(&self) -> &FMultisizeIndexContainer {
        &self.indices
    }

    pub fn get_static_buffer(&self) -> &FStaticMeshVertexBuffer {
        match &self.static_mesh_vertex_buffer {
            Some(buffer) => buffer,
            None => panic!("No static mesh buffer found. Cannot do mesh conversion."),
        }
    }

    pub fn get_weight_buffer(&self) -> &FSkinWeightVertexBuffer {
        match &self.skin_weight_vertex_buffer {
            Some(buffer) => buffer,
            None => panic!("No weight buffer found. Cannot do mesh conversion."),
        }
    }

    pub fn get_sections(&self) -> &Vec<FSkelMeshRenderSection> {
        &self.sections
    }

    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, has_vertex_colors: bool) -> ParserResult<Self> {
        let flags = FStripDataFlags::new(reader)?;
        let sections = read_tarray_n(reader, name_map, import_map)?;
        let indices = FMultisizeIndexContainer::new(reader)?;
        let active_bone_indices = read_tarray(reader)?;
        let required_bones = read_tarray(reader)?;

        let render_data = !flags.is_data_stripped_for_server() && !flags.is_class_data_stripped(2);
        if !render_data {
            return Err(ParserError::new(format!("Could not read FSkelMesh, no renderable data")));
        }
        let position_vertex_buffer = FPositionVertexBuffer::new(reader)?;
        let static_mesh_vertex_buffer = FStaticMeshVertexBuffer::new(reader)?;
        let skin_weight_vertex_buffer = FSkinWeightVertexBuffer::new(reader)?;

        let colour_vertex_buffer = match has_vertex_colors {
            true => {
                Some(FColorVertexBuffer::new(reader)?)
            },
            false => None,
        };

        if flags.is_class_data_stripped(1) {

        }

        Ok(Self {
            sections, indices, active_bone_indices, required_bones, position_vertex_buffer,
            static_mesh_vertex_buffer, skin_weight_vertex_buffer, colour_vertex_buffer,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMeshToMeshVertData {
    position_bary_coords: FVector4,
    normal_bary_coords: FVector4,
    tangent_bary_coords: FVector4,
    source_mesh_vert_indices: [u16;4],
    padding: [u32; 2],
}

impl Newable for FMeshToMeshVertData {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let position_bary_coords = FVector4::new(reader)?;
        let normal_bary_coords = FVector4::new(reader)?;
        let tangent_bary_coords = FVector4::new(reader)?;
        let mut source_mesh_vert_indices = [0u16;4];
        for i in 0..4 {
            source_mesh_vert_indices[i] = reader.read_u16::<LittleEndian>()?;
        }
        let mut padding = [0u32;2];
        for i in 0..2 {
            padding[i] = reader.read_u32::<LittleEndian>()?;
        }

        Ok(Self {
            position_bary_coords, normal_bary_coords, tangent_bary_coords, source_mesh_vert_indices, padding,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct USkeletalMesh {
    super_object: UObject,
    imported_bounds: FBoxSphereBounds,
    materials: Vec<FSkeletalMaterial>,
    ref_skeleton: FReferenceSkeleton,
    lod_models: Vec<FSkeletalMeshRenderData>,
}

impl USkeletalMesh {
    pub(super) fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "SkeletalMesh")?;
        let has_vertex_colors = match super_object.get_property("bHasVertexColors") {
            Some(property_data) => {
                match property_data {
                    FPropertyTagType::BoolProperty(property_bool) => *property_bool,
                    _ => false,
                }
            },
            None => false,
        };
        let flags = FStripDataFlags::new(reader)?;
        let imported_bounds = FBoxSphereBounds::new(reader)?;
        let materials: Vec<FSkeletalMaterial> = read_tarray_n(reader, name_map, import_map)?;
        let ref_skeleton = FReferenceSkeleton::new_n(reader, name_map, import_map)?;

        if !flags.is_editor_data_stripped() {
            println!("editor data still present");
        }

        let cooked = reader.read_u32::<LittleEndian>()? != 0;
        if !cooked {
            return Err(ParserError::new(format!("Asset does not contain cooked data.")));
        }
        let num_models = reader.read_u32::<LittleEndian>()?;
        let mut lod_models = Vec::new();
        for _i in 0..num_models {
            lod_models.push(FSkeletalMeshRenderData::new(reader, name_map, import_map, has_vertex_colors)?);
        }

        let _serialize_guid = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            super_object, imported_bounds, materials, ref_skeleton, lod_models,
        })
    }

    pub fn get_first_lod(&self) -> &FSkeletalMeshRenderData {
        self.lod_models.get(0).unwrap()
    }

    pub fn get_materials(&self) -> &Vec<FSkeletalMaterial> {
        &self.materials
    }

    pub fn get_skeleton(&self) -> &FReferenceSkeleton {
        &self.ref_skeleton
    }
}

impl PackageExport for USkeletalMesh {
    fn get_export_type(&self) -> &str {
        "get_export_type"
    }
}