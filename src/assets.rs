use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::fs::{File, metadata};
use std::path::Path;
use std::any::Any;
use std::sync::Arc;
use serde::Serialize;
use serde::ser::{Serializer, SerializeMap, SerializeSeq, SerializeStruct};
use serde_json::error::Error as JSONError;
use erased_serde::{serialize_trait_object, Serialize as TraitSerialize};
use byteorder::{LittleEndian, ReadBytesExt};
use bit_vec::BitVec;
use lazy_static::lazy_static;
use crate::mapping::{MappingStore, PropertyMapping, TagMapping};
use crate::dispatch::{LoaderGlobalData, InitialLoadMetaData, FNameMap};

pub mod locale;
// mod material_instance;
// mod anims;
// mod meshes;
// mod sound;

// pub use anims::{USkeleton, UAnimSequence, FTrack};
// pub use meshes::{USkeletalMesh, FMultisizeIndexContainer, FStaticMeshVertexDataTangent, FSkeletalMeshRenderData,
//     FSkelMeshRenderSection, FSkeletalMaterial, FSkinWeightVertexBuffer, FMeshBoneInfo, FStaticMeshVertexDataUV, FReferenceSkeleton};
// pub use sound::USoundWave;

lazy_static! {
    static ref MAPPINGS: MappingStore = MappingStore::build_mappings().unwrap();
}

pub type ReaderCursor<'c> = Cursor<&'c[u8]>;

#[derive(Debug, Serialize)]
pub enum ParserType {
    Unknown,
    ClassMappingMissing,
    PropertyIndexMissing,
}

/// ParserError contains a list of error messages that wind down to where the parser was not able to parse a property
#[derive(Debug, Serialize)]
pub struct ParserError {
    property_list: Vec<String>,
    error_type: ParserType,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#?}", self.property_list)
    }
}

impl ParserError {
    pub fn new(start: String) -> Self {
        Self {
            property_list: vec![start],
            error_type: ParserType::Unknown,
        }
    }

    pub fn add(mut error: ParserError, property: String) -> Self {
        error.property_list.push(property);
        error
    }

    pub fn get_properties(&self) -> &Vec<String> {
        &self.property_list
    }

    pub fn typed(start: String, error_type: ParserType) -> Self {
        Self {
            property_list: vec![start],
            error_type,
        }
    }

    pub fn get_type(&self) -> &ParserType {
        &self.error_type
    }
}

impl From<std::io::Error> for ParserError {
    fn from(error: std::io::Error) -> ParserError {
        ParserError::new(format!("File Error: {}", error))
    }
}

impl From<std::str::Utf8Error> for ParserError {
    fn from(_error: std::str::Utf8Error) -> ParserError {
        ParserError::new("UTF8 Error".to_owned())
    }
}

impl From<std::string::FromUtf16Error> for ParserError {
    fn from(_error: std::string::FromUtf16Error) -> ParserError {
        ParserError::new("UTF16 Error".to_owned())
    }
}

impl From<JSONError> for ParserError {
    fn from(error: JSONError) -> ParserError {
        ParserError::new(format!("JSON Error: {}", error))
    }
}

impl std::error::Error for ParserError { }

pub type ParserResult<T> = Result<T, ParserError>;

pub trait Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> where Self: Sized;
}

#[derive(Debug, PartialEq, Clone)]
pub struct FGuid {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

impl Newable for FGuid {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            a: reader.read_u32::<LittleEndian>()?,
            b: reader.read_u32::<LittleEndian>()?,
            c: reader.read_u32::<LittleEndian>()?,
            d: reader.read_u32::<LittleEndian>()?,
        })
    }
}

impl NewableWithNameMap for FGuid {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        FGuid::new(reader)
    }
}

impl fmt::Display for FGuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:08x}{:08x}{:08x}{:08x}", self.a, self.b, self.c, self.d)
    }
}

impl Serialize for FGuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
struct FCustomVersion {
    key: FGuid,
    version: i32,
}

impl Newable for FCustomVersion {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            key: FGuid::new(reader)?,
            version: reader.read_i32::<LittleEndian>()?,
        })
    }
}

pub fn read_short_string(reader: &mut ReaderCursor) -> ParserResult<String> {
    let data1 = reader.read_u8()? as u32;
    let data2 = reader.read_u8()? as u32;

    let length: u32 = ((data1 & 0x007F) << 8) + data2;
    let utf16 = (data1 & 0x0080) != 0;

    let fstr;

    if utf16 {
        let mut u16bytes = vec![0u16; length as usize];
        for i in 0..length {
            let val = reader.read_u16::<LittleEndian>()?;
            u16bytes[i as usize] = val;
        }
        fstr = String::from_utf16(&u16bytes)?;
    } else {
        let mut bytes = vec![0u8; length as usize];
        reader.read_exact(&mut bytes)?;
        fstr = std::str::from_utf8(&bytes)?.to_owned();
    }

    Ok(fstr)
}

pub fn read_string(reader: &mut ReaderCursor) -> ParserResult<String> {
    let mut length = reader.read_i32::<LittleEndian>()?;
    if length > 65536 || length < -65536 {
        return Err(ParserError::new(format!("String length too large ({}), likely a read error.", length)));
    }

    if length == 0 {
        return Ok("".to_owned());
    }

    let mut fstr;

    if length < 0 {
        length *= -1;
        let mut u16bytes = vec![0u16; length as usize];
        for i in 0..length {
            let val = reader.read_u16::<LittleEndian>()?;
            u16bytes[i as usize] = val;
        }
        u16bytes.pop();
        fstr = String::from_utf16(&u16bytes)?;
    } else {
        let mut bytes = vec![0u8; length as usize];
        reader.read_exact(&mut bytes)?;
        fstr = std::str::from_utf8(&bytes)?.to_owned();
        fstr.pop();
    }

    Ok(fstr)
}

#[derive(Debug)]
struct FGenerationInfo {
    export_count: i32,
    name_count: i32,
}

impl Newable for FGenerationInfo {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            export_count: reader.read_i32::<LittleEndian>()?,
            name_count: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FEngineVersion {
    major: u16,
    minor: u16,
    patch: u16,
    changelist: u32,
    branch: String,
}

impl FEngineVersion {
    fn empty() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            changelist: 0,
            branch: "".to_owned(),
        }
    }
}

impl Newable for FEngineVersion {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            major: reader.read_u16::<LittleEndian>()?,
            minor: reader.read_u16::<LittleEndian>()?,
            patch: reader.read_u16::<LittleEndian>()?,
            changelist: reader.read_u32::<LittleEndian>()?,
            branch: read_string(reader)?,
        })
    }
}

pub fn read_tarray<S>(reader: &mut ReaderCursor) -> ParserResult<Vec<S>> where S: Newable {
    let length = reader.read_u32::<LittleEndian>()?;
    let mut container = Vec::new();

    for _i in 0..length {
        container.push(S::new(reader)?);
    }

    Ok(container)
}

fn read_tarray_n<S>(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Vec<S>> where S: NewableWithNameMap {
    let length = reader.read_u32::<LittleEndian>()?;
    let mut container = Vec::new();

    for _i in 0..length {
        container.push(S::new_n(reader, name_map, import_map)?);
    }

    Ok(container)
}

impl Newable for u8 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_u8()?)
    }
}

impl Newable for String {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        read_string(reader)
    }
}

impl Newable for u32 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_u32::<LittleEndian>()?)
    }
}

impl Newable for u64 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_u64::<LittleEndian>()?)
    }
}

impl Newable for i32 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_i32::<LittleEndian>()?)
    }
}

impl Newable for f32 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_f32::<LittleEndian>()?)
    }
}

impl Newable for u16 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_u16::<LittleEndian>()?)
    }
}

impl Newable for i16 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(reader.read_i16::<LittleEndian>()?)
    }
}

impl NewableWithNameMap for String {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        read_fname(reader, name_map)
    }
}

#[derive(Debug, Serialize)]
enum TRangeBoundType {
    RangeExclusive,
    RangeInclusive,
    RangeOpen,
}

#[derive(Debug, Serialize)]
struct TRangeBound<T> {
    bound_type: TRangeBoundType,
    value: T,
}

impl<T> Newable for TRangeBound<T> where T: Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let bound_type = reader.read_u8()?;
        let bound_type = match bound_type {
            0 => TRangeBoundType::RangeExclusive,
            1 => TRangeBoundType::RangeInclusive,
            2 => TRangeBoundType::RangeOpen,
            _ => panic!("Range bound type not supported"),
        };

        let value = T::new(reader)?;

        Ok(Self {
            bound_type, value
        })
    }
}

#[derive(Debug, Serialize)]
struct TRange<T> {
    lower_bound: TRangeBound<T>,
    upper_bound: TRangeBound<T>,
}

impl<T> Newable for TRange<T> where T: Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            lower_bound: TRangeBound::new(reader)?,
            upper_bound: TRangeBound::new(reader)?,
        })
    }
}

#[derive(Debug)]
struct FCompressedChunk {
    uncompressed_offset: i32,
    uncompressed_size: i32,
    compressed_offset: i32,
    compressed_size: i32,
}

impl Newable for FCompressedChunk {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            uncompressed_offset: reader.read_i32::<LittleEndian>()?,
            uncompressed_size: reader.read_i32::<LittleEndian>()?,
            compressed_offset: reader.read_i32::<LittleEndian>()?,
            compressed_size: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FPackageFileSummary {
    tag: i32,
    legacy_file_version: i32,
    legacy_ue3_version: i32,
    file_version_u34: i32,
    file_version_licensee_ue4: i32,
    custom_version_container: Vec<FCustomVersion>,
    total_header_size: i32,
    folder_name: String,
    package_flags: u32,
    name_count: i32,
    name_offset: i32,
    gatherable_text_data_count: i32,
    gatherable_text_data_offset: i32,
    export_count: i32,
    export_offset: i32,
    import_count: i32,
    import_offset: i32,
    depends_offset: i32,
    string_asset_references_count: i32,
    string_asset_references_offset: i32,
    searchable_names_offset: i32,
    thumbnail_table_offset: i32,
    guid: FGuid,
    generations: Vec<FGenerationInfo>,
    saved_by_engine_version: FEngineVersion,
    compatible_with_engine_version: FEngineVersion,
    compression_flags: u32,
    compressed_chunks: Vec<FCompressedChunk>,
    package_source: u32,
    additional_packages_to_cook: Vec<String>,
    asset_registry_data_offset: i32,
    buld_data_start_offset: i32,
    world_tile_info_data_offset: i32,
    chunk_ids: Vec<i32>,
    preload_dependency_count: i32,
    preload_dependency_offset: i32,
}

impl FPackageFileSummary {
    fn empty() -> Self {
        Self {
            tag: 0,
            legacy_file_version: 0,
            legacy_ue3_version: 0,
            file_version_u34: 0,
            file_version_licensee_ue4: 0,
            custom_version_container: Vec::new(),
            total_header_size: 0,
            folder_name: "".to_owned(),
            package_flags: 0,
            name_count: 0,
            name_offset: 0,
            gatherable_text_data_count: 0,
            gatherable_text_data_offset: 0,
            export_count: 0,
            export_offset: 0,
            import_count: 0,
            import_offset: 0,
            depends_offset: 0,
            string_asset_references_count: 0,
            string_asset_references_offset: 0,
            searchable_names_offset: 0,
            thumbnail_table_offset: 0,
            guid: FGuid {a: 0, b: 0, c: 0, d: 0},
            generations: Vec::new(),
            saved_by_engine_version: FEngineVersion::empty(),
            compatible_with_engine_version: FEngineVersion::empty(),
            compression_flags: 0,
            compressed_chunks: Vec::new(),
            package_source: 0,
            additional_packages_to_cook: Vec::new(),
            asset_registry_data_offset: 0,
            buld_data_start_offset: 0,
            world_tile_info_data_offset: 0,
            chunk_ids: Vec::new(),
            preload_dependency_count: 0,
            preload_dependency_offset: 0,
        }
    }
}

impl Newable for FPackageFileSummary {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            tag: reader.read_i32::<LittleEndian>()?,
            legacy_file_version: reader.read_i32::<LittleEndian>()?,
            legacy_ue3_version: reader.read_i32::<LittleEndian>()?,
            file_version_u34: reader.read_i32::<LittleEndian>()?,
            file_version_licensee_ue4: reader.read_i32::<LittleEndian>()?,
            custom_version_container: read_tarray(reader)?,
            total_header_size: reader.read_i32::<LittleEndian>()?,
            folder_name: read_string(reader)?,
            package_flags: reader.read_u32::<LittleEndian>()?,
            name_count: reader.read_i32::<LittleEndian>()?,
            name_offset: reader.read_i32::<LittleEndian>()?,
            gatherable_text_data_count: reader.read_i32::<LittleEndian>()?,
            gatherable_text_data_offset: reader.read_i32::<LittleEndian>()?,
            export_count: reader.read_i32::<LittleEndian>()?,
            export_offset: reader.read_i32::<LittleEndian>()?,
            import_count: reader.read_i32::<LittleEndian>()?,
            import_offset: reader.read_i32::<LittleEndian>()?,
            depends_offset: reader.read_i32::<LittleEndian>()?,
            string_asset_references_count: reader.read_i32::<LittleEndian>()?,
            string_asset_references_offset: reader.read_i32::<LittleEndian>()?,
            searchable_names_offset: reader.read_i32::<LittleEndian>()?,
            thumbnail_table_offset: reader.read_i32::<LittleEndian>()?,
            guid: FGuid::new(reader)?,
            generations: read_tarray(reader)?,
            saved_by_engine_version: FEngineVersion::new(reader)?,
            compatible_with_engine_version: FEngineVersion::new(reader)?,
            compression_flags: reader.read_u32::<LittleEndian>()?,
            compressed_chunks: read_tarray(reader)?,
            package_source: reader.read_u32::<LittleEndian>()?,
            additional_packages_to_cook: read_tarray(reader)?,
            asset_registry_data_offset: reader.read_i32::<LittleEndian>()?,
            buld_data_start_offset: reader.read_i32::<LittleEndian>()?,
            world_tile_info_data_offset: reader.read_i32::<LittleEndian>()?,
            chunk_ids: read_tarray(reader)?,
            preload_dependency_count: reader.read_i32::<LittleEndian>()?,
            preload_dependency_offset: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
enum FMappedNameType {
    Package,
    Container,
    Global,
}

#[derive(Debug)]
pub struct FMappedName {
    index: u32,
    number: u32,
    name_type: FMappedNameType,
}

impl FMappedName {
    pub fn get_name<'a>(&self, map: &'a FNameMap) -> ParserResult<&'a str> {
        map.get_name(self.index as usize)
    }
}

impl Newable for FMappedName {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let index = reader.read_u32::<LittleEndian>()?;
        let number = reader.read_u32::<LittleEndian>()?;

        let index_mask = (1 << 30) - 1;
        let type_mask = !index_mask;
        let name_type = match (index & type_mask) >> 30 {
            0 => FMappedNameType::Package,
            1 => FMappedNameType::Container,
            2 => FMappedNameType::Global,
            _ => panic!("No name type"),
        };

        Ok(Self {
            index: index & index_mask,
            number,
            name_type,
        })
    }
}

#[derive(Debug)]
struct FPackageSummary {
    name: FMappedName,
    source_name: FMappedName,
    package_flags: u32,
    header_size: u32,
    name_map_offset: i32,
    name_map_size: i32,
    name_map_hash_ofsset: i32,
    name_map_hash_size: i32,
    import_map_offset: i32,
    export_map_offset: i32,
    export_bundle_offset: i32,
    graph_data_offset: i32,
    graph_data_size: i32,
    pad: i32,
}

impl FPackageSummary {
    fn empty() -> Self {
        Self {
            name: FMappedName {index: 0, number: 0, name_type: FMappedNameType::Package},
            source_name: FMappedName {index: 0, number: 0, name_type: FMappedNameType::Package},
            package_flags: 0,
            header_size: 0,
            name_map_offset: 0,
            name_map_size: 0,
            name_map_hash_ofsset: 0,
            name_map_hash_size: 0,
            import_map_offset: 0,
            export_map_offset: 0,
            export_bundle_offset: 0,
            graph_data_offset: 0,
            graph_data_size: 0,
            pad: 0,
        }
    }
}

impl Newable for FPackageSummary {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            name: FMappedName::new(reader)?,
            source_name: FMappedName::new(reader)?,
            package_flags: reader.read_u32::<LittleEndian>()?,
            header_size: reader.read_u32::<LittleEndian>()?,
            name_map_offset: reader.read_i32::<LittleEndian>()?,
            name_map_size: reader.read_i32::<LittleEndian>()?,
            name_map_hash_ofsset: reader.read_i32::<LittleEndian>()?,
            name_map_hash_size: reader.read_i32::<LittleEndian>()?,
            import_map_offset: reader.read_i32::<LittleEndian>()?,
            export_map_offset: reader.read_i32::<LittleEndian>()?,
            export_bundle_offset: reader.read_i32::<LittleEndian>()?,
            graph_data_offset: reader.read_i32::<LittleEndian>()?,
            graph_data_size: reader.read_i32::<LittleEndian>()?,
            pad: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum FPackageObjectIndex_Type {
    Export,
    ScriptImport,
    PackageImport,
    Null
}

#[derive(Debug, Clone, PartialEq)]
pub struct FPackageObjectIndex {
    index: u64,
    index_type: FPackageObjectIndex_Type,
}

impl FPackageObjectIndex {
    fn get_export_name<'a>(&self, name_map: &'a NameMap, import_map: &'a ImportMap) -> ParserResult<&'a str> {
        match self.index_type {
            FPackageObjectIndex_Type::ScriptImport => match import_map.global.get_package_name(&self, &name_map.global) {
                Some(n) => Ok(n),
                None => return Err(ParserError::new(format!("No package class found"))),
            },
            _ => return Err(ParserError::new(format!("Unknown Import Type"))),
        }
    }

    pub fn get_index(&self) -> u64 {
        self.index
    }
}

impl Newable for FPackageObjectIndex {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let data = reader.read_u64::<LittleEndian>()?;

        let index_mask: u64 = (1 << 62) - 1;
        let index = data & index_mask;

        let index_type = match data >> 62 {
            0 => FPackageObjectIndex_Type::Export,
            1 => FPackageObjectIndex_Type::ScriptImport,
            2 => FPackageObjectIndex_Type::PackageImport,
            _ => FPackageObjectIndex_Type::Null,
        };

        Ok(Self {
            index, index_type,
        })
    }
}

impl Serialize for FPackageObjectIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.index.to_string())
    }
}

#[derive(Debug)]
struct FNameEntrySerialized {
    data: String,
    non_case_preserving_hash: u16,
    case_preserving_hash: u16,
}

impl Newable for FNameEntrySerialized {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            data: read_string(reader)?,
            non_case_preserving_hash: reader.read_u16::<LittleEndian>()?,
            case_preserving_hash: reader.read_u16::<LittleEndian>()?,
        })
    }
}

struct NameMap {
    names: FNameMap,
    global: Arc<FNameMap>,
}

struct ImportMap {
    imports: Vec<FPackageObjectIndex>,
    global: Arc<InitialLoadMetaData>,
}

trait NewableWithNameMap: std::fmt::Debug + TraitSerialize {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self>
    where Self: Sized;

    // This seems ridiculous... but there's no way I'm satisifying the requirements for Any on this trait
    fn get_properties(&self) -> ParserResult<&Vec<FPropertyTag>> {
        Err(ParserError::new(format!("Not implemented for this type")))
    }
}

serialize_trait_object!(NewableWithNameMap);

fn read_fname(reader: &mut ReaderCursor, name_map: &NameMap) -> ParserResult<String> {
    let mapped_name = FMappedName::new(reader)?;
    Ok(mapped_name.get_name(&name_map.names)?.to_owned())
}

#[derive(Debug, Clone)]
pub struct FPackageIndex {
    index: i32,
    import: Option<FPackageObjectIndex>,
}

impl FPackageIndex {
    fn get_package(index: i32, import_map: &ImportMap) -> Option<FPackageObjectIndex> {
        if index < 0 {
            return match import_map.imports.get((index * -1 - 1) as usize) {
                Some(data) => Some(data.clone()),
                None => None,
            };
        }
        None
    }

    pub fn get_import(&self) -> &Option<FPackageObjectIndex> {
        &self.import
    }
}

impl NewableWithNameMap for FPackageIndex {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let import = FPackageIndex::get_package(index, import_map);
        
        Ok(Self {
            index,
            import,
        })
    }
}

impl Serialize for FPackageIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        if self.index >= 0 {
            let mut state = serializer.serialize_struct("FObjectExport", 1)?;
            state.serialize_field("export", &self.index)?;
            state.end()
        } else {
            let mut state = serializer.serialize_struct("FPackageObjectIndex", 1)?;
            state.serialize_field("import", &self.import)?;
            state.end()
        }
    }
}

#[derive(Debug)]
struct FObjectExport {
    class_index: FPackageIndex,
    super_index: FPackageIndex,
    template_index: FPackageIndex,
    outer_index: FPackageIndex,
    object_name: String,
    save: u32,
    serial_size: i64,
    serial_offset: i64,
    forced_export: bool,
    not_for_client: bool,
    not_for_server: bool,
    package_guid: FGuid,
    package_flags: u32,
    not_always_loaded_for_editor_game: bool,
    is_asset: bool,
    first_export_dependency: i32,
    serialization_before_serialization_dependencies: bool,
    create_before_serialization_dependencies: bool,
    serialization_before_create_dependencies: bool,
    create_before_create_dependencies: bool,
}

impl NewableWithNameMap for FObjectExport {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            class_index: FPackageIndex::new_n(reader, name_map, import_map)?,
            super_index: FPackageIndex::new_n(reader, name_map, import_map)?,
            template_index: FPackageIndex::new_n(reader, name_map, import_map)?,
            outer_index: FPackageIndex::new_n(reader, name_map, import_map)?,
            object_name: read_fname(reader, name_map)?,
            save: reader.read_u32::<LittleEndian>()?,
            serial_size: reader.read_i64::<LittleEndian>()?,
            serial_offset: reader.read_i64::<LittleEndian>()?,
            forced_export: reader.read_i32::<LittleEndian>()? != 0,
            not_for_client: reader.read_i32::<LittleEndian>()? != 0,
            not_for_server: reader.read_i32::<LittleEndian>()? != 0,
            package_guid: FGuid::new(reader)?,
            package_flags: reader.read_u32::<LittleEndian>()?,
            not_always_loaded_for_editor_game: reader.read_i32::<LittleEndian>()? != 0,
            is_asset: reader.read_i32::<LittleEndian>()? != 0,
            first_export_dependency: reader.read_i32::<LittleEndian>()?,
            serialization_before_serialization_dependencies: reader.read_i32::<LittleEndian>()? != 0,
            create_before_serialization_dependencies: reader.read_i32::<LittleEndian>()? != 0,
            serialization_before_create_dependencies: reader.read_i32::<LittleEndian>()? != 0,
            create_before_create_dependencies: reader.read_i32::<LittleEndian>()? != 0,
        })
    }
}

impl Serialize for FObjectExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.object_name)
    }
}

#[derive(Debug)]
struct FExportMapEntry {
    serial_offset: u64,
    serial_size: u64,
    object_name: FMappedName,
    outer_index: FPackageObjectIndex,
    class_index: FPackageObjectIndex,
    super_index: FPackageObjectIndex,
    template_index: FPackageObjectIndex,
    global_import_index: FPackageObjectIndex,
    object_flags: u32,
    filter_flags: u8,
}

impl FExportMapEntry {
    fn get_export_name<'a>(&self, name_map: &'a NameMap, import_map: &'a ImportMap) -> ParserResult<&'a str> {
        match self.class_index.index_type {
            FPackageObjectIndex_Type::ScriptImport => self.class_index.get_export_name(name_map, import_map),
            FPackageObjectIndex_Type::PackageImport => self.get_object_name(name_map),
            _ => Err(ParserError::new(format!("Unknown Export Map Type"))),
        }
    }

    fn get_object_name<'a>(&self, name_map: &'a NameMap) -> ParserResult<&'a str> {
        self.object_name.get_name(&name_map.names)
    }
}

impl Newable for FExportMapEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let res = Self {
            serial_offset: reader.read_u64::<LittleEndian>()?,
            serial_size: reader.read_u64::<LittleEndian>()?,
            object_name: FMappedName::new(reader)?,
            outer_index: FPackageObjectIndex::new(reader)?,
            class_index: FPackageObjectIndex::new(reader)?,
            super_index: FPackageObjectIndex::new(reader)?,
            template_index: FPackageObjectIndex::new(reader)?,
            global_import_index: FPackageObjectIndex::new(reader)?,
            object_flags: reader.read_u32::<LittleEndian>()?,
            filter_flags: reader.read_u8()?,
        };

        let mut data = [0u8; 3];
        reader.read_exact(&mut data)?;

        Ok(res)
    }
}

#[derive(Debug)]
pub struct FText {
    flags: u32,
    history_type: i8,
    namespace: String,
    key: String,
    source_string: String,
    invariant: String,
}

impl Newable for FText {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let flags = reader.read_u32::<LittleEndian>()?;
        let history_type = reader.read_i8()?;

        match history_type {
            -1 => {
                let invariant = reader.read_u32::<LittleEndian>()? != 0;
                Ok(Self {
                    flags,
                    history_type,
                    namespace: "".to_owned(),
                    key: "".to_owned(),
                    source_string: "".to_owned(),
                    invariant: if invariant { read_string(reader)? } else { "".to_owned() } 
                })
            },
            0 => Ok(Self {
                flags,
                history_type,
                namespace: read_string(reader)?,
                key: read_string(reader)?,
                source_string: read_string(reader)?,
                invariant: "".to_owned()
            }),
            _ => Err(ParserError::new(format!("Could not read history type: {}", history_type))),
        }
    }
}

impl Serialize for FText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("FText", 3)?;
        state.serialize_field("string", &self.source_string)?;
        state.serialize_field("namespace", &self.namespace)?;
        state.serialize_field("key", &self.key)?;
        state.end()
    }
}

#[derive(Debug, Serialize)]
pub struct FSoftObjectPath {
    asset_path_name: String,
    sub_path_string: String,
}

impl NewableWithNameMap for FSoftObjectPath {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            asset_path_name: read_fname(reader, name_map)?,
            sub_path_string: read_string(reader)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FGameplayTagContainer {
    gameplay_tags: Vec<String>,
}

impl NewableWithNameMap for FGameplayTagContainer {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        let length = reader.read_u32::<LittleEndian>()?;
        let mut container = Vec::new();

        for _i in 0..length {
            container.push(read_fname(reader, name_map)?);
        }

        Ok(Self {
            gameplay_tags: container,
        })
    }
}

#[derive(Debug, Serialize)]
struct FIntPoint {
    x: u32,
    y: u32,
}

impl NewableWithNameMap for FIntPoint {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_u32::<LittleEndian>()?,
            y: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize, Copy, Clone)]
pub struct FVector2D {
    x: f32,
    y: f32,
}

impl Newable for FVector2D {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_f32::<LittleEndian>()?,
            y: reader.read_f32::<LittleEndian>()?,
        })
    }
}

impl FVector2D {
    pub fn get_tuple(&self) -> (f32, f32) {
        (self.x, self.y)
    }
}

impl NewableWithNameMap for FVector2D {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

#[derive(Debug, Serialize)]
struct FLinearColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl NewableWithNameMap for FLinearColor {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            r: reader.read_f32::<LittleEndian>()?,
            g: reader.read_f32::<LittleEndian>()?,
            b: reader.read_f32::<LittleEndian>()?,
            a: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Newable for FColor {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            b: reader.read_u8()?,
            g: reader.read_u8()?,
            r: reader.read_u8()?,
            a: reader.read_u8()?,
        })
    }
}

impl NewableWithNameMap for FColor {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

#[derive(Debug)]
struct FStructFallback {
    properties: Vec<FPropertyTag>,
}

impl NewableWithNameMap for FStructFallback {
    fn new_n(_reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        panic!("Unimplemented");
    }

    fn get_properties(&self) -> ParserResult<&Vec<FPropertyTag>> {
        Ok(&self.properties)
    }
}

impl FStructFallback {
    fn new_unversioned(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, struct_type: &str) -> ParserResult<Self> {
        let object = UObject::new(reader, name_map, import_map, struct_type, None)?;
        Ok(Self {
            properties: object.properties,
        })
    }
}

impl Serialize for FStructFallback {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.properties.len()))?;
        for property in &self.properties {
            map.serialize_entry(&property.name, &property.tag)?;
        }
        map.end()
    }
}

#[derive(Debug)]
pub struct UScriptStruct {
    struct_name: String,
    struct_type: Box<dyn NewableWithNameMap>,
}

#[derive(Debug, Serialize)]
struct FLevelSequenceLegacyObjectReference {
    key_guid: FGuid,
    object_id: FGuid,
    object_path: String,
}

impl Newable for FLevelSequenceLegacyObjectReference {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            key_guid: FGuid::new(reader)?,
            object_id: FGuid::new(reader)?,
            object_path: read_string(reader)?,
        })
    }
}

#[derive(Debug)]
struct FLevelSequenceObjectReferenceMap {
    map_data: Vec<FLevelSequenceLegacyObjectReference>,
}

impl NewableWithNameMap for FLevelSequenceObjectReferenceMap {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        let mut map_data = Vec::new();
        let element_count = reader.read_i32::<LittleEndian>()?;
        for _i in 0..element_count {
            map_data.push(FLevelSequenceLegacyObjectReference::new(reader)?);
        }
        Ok(Self {
            map_data
        })
    }
}

impl Serialize for FLevelSequenceObjectReferenceMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.map_data.len()))?;
        for property in &self.map_data {
            map.serialize_entry(&property.key_guid.to_string(), &property.object_path)?;
        }
        map.end()
    }
}

#[derive(Debug, Serialize)]
struct FMovieSceneSegment {
    range: TRange<i32>,
    id: i32,
    allow_empty: bool,
    impls: Vec<UScriptStruct>,
}

impl NewableWithNameMap for FMovieSceneSegment {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let range: TRange<i32> = TRange::new(reader)?;
        let id = reader.read_i32::<LittleEndian>()?;
        let allow_empty = reader.read_u32::<LittleEndian>()? != 0;
        let num_structs = reader.read_u32::<LittleEndian>()?;
        let mut impls: Vec<UScriptStruct> = Vec::new();
        for _i in 0..num_structs {
            impls.push(UScriptStruct::new(reader, name_map, import_map, "SectionEvaluationData")?);
        }
        Ok(Self {
            range, id, allow_empty, impls,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMovieSceneEvaluationTreeNode {
    range: TRange<i32>,
    parent: FMovieSceneEvaluationTreeNodeHandle,
    children_id: FEvaluationTreeEntryHandle,
    data_id: FEvaluationTreeEntryHandle,

}

impl Newable for FMovieSceneEvaluationTreeNode {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            range: TRange::new(reader)?,
            parent: FMovieSceneEvaluationTreeNodeHandle::new(reader)?,
            children_id: FEvaluationTreeEntryHandle::new(reader)?,
            data_id: FEvaluationTreeEntryHandle::new(reader)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMovieSceneEvaluationTreeNodeHandle {
    children_handle: FEvaluationTreeEntryHandle,
    index: i32,
}

impl Newable for FMovieSceneEvaluationTreeNodeHandle {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            children_handle: FEvaluationTreeEntryHandle::new(reader)?,
            index: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FEvaluationTreeEntryHandle {
    entry_index: i32,
}

impl Newable for FEvaluationTreeEntryHandle {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            entry_index: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct TEvaluationTreeEntryContainer<T> {
    entries: Vec<FEntry>,
    items: Vec<T>,
}

impl<T> Newable for TEvaluationTreeEntryContainer<T> where T: Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            entries: read_tarray(reader)?,
            items: read_tarray(reader)?,
        })
    }
}

impl<T> NewableWithNameMap for TEvaluationTreeEntryContainer<T> where T: NewableWithNameMap + Serialize {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            entries: read_tarray(reader)?,
            items: read_tarray_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMovieSceneEvaluationTree {
    root_node: FMovieSceneEvaluationTreeNode,
    child_nodes: TEvaluationTreeEntryContainer<FMovieSceneEvaluationTreeNode>,
}

impl Newable for FMovieSceneEvaluationTree {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            root_node: FMovieSceneEvaluationTreeNode::new(reader)?,
            child_nodes: TEvaluationTreeEntryContainer::new(reader)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FEntry {
    start_index: i32,
    size: i32,
    capacity: i32,
}

impl Newable for FEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            start_index: reader.read_i32::<LittleEndian>()?,
            size: reader.read_i32::<LittleEndian>()?,
            capacity: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct TMovieSceneEvaluationTree<T> {
    base_tree: FMovieSceneEvaluationTree,
    data: TEvaluationTreeEntryContainer<T>,
}

impl<T> NewableWithNameMap for TMovieSceneEvaluationTree<T> where T: NewableWithNameMap + Serialize {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            base_tree: FMovieSceneEvaluationTree::new(reader)?,
            data: TEvaluationTreeEntryContainer::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FSectionEvaluationDataTree {
    tree: TMovieSceneEvaluationTree<FStructFallback>,
}

impl NewableWithNameMap for FSectionEvaluationDataTree {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            tree: TMovieSceneEvaluationTree::new_n(reader, name_map, import_map)?,
        })
    }
}

// wat
#[derive(Debug, Serialize)]
struct InlineUStruct {
    type_name: String,
    data: FStructFallback,
}

impl NewableWithNameMap for InlineUStruct {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let type_name = read_string(reader)?;
        Ok(Self {
            type_name,
            data: FStructFallback::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMovieSceneFrameRange {
    value: TRange<i32>,
}

impl NewableWithNameMap for FMovieSceneFrameRange {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            value: TRange::new(reader)?,
        })
    }
}

// There are too many types that are just i32s. This is a replacement for those.
#[derive(Debug)]
struct FI32 {
    value: i32,
}

impl NewableWithNameMap for FI32 {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            value: reader.read_i32::<LittleEndian>()?,
        })
    }
}

impl Serialize for FI32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_i32(self.value)
    }
}

#[derive(Debug)]
struct FU32 {
    value: u32,
}

impl NewableWithNameMap for FU32 {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            value: reader.read_u32::<LittleEndian>()?,
        })
    }
}

impl Serialize for FU32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_u32(self.value)
    }
}

#[derive(Debug, Serialize)]
struct FMovieSceneEvaluationKey {
    sequence_id: u32,
    track_identifier: i32,
    section_index: u32,
}

impl NewableWithNameMap for FMovieSceneEvaluationKey {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            sequence_id: reader.read_u32::<LittleEndian>()?,
            track_identifier: reader.read_i32::<LittleEndian>()?,
            section_index: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FQuat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

impl FQuat {
    pub fn get_tuple(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.z, self.w)
    }

    fn new_raw(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            x, y, z, w,
        }
    }

    fn unit() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    fn rebuild_w(&mut self) {
        let ww = 1.0 - (self.x*self.x + self.y*self.y + self.z*self.z);
        self.w = match ww > 0.0 {
            true => ww.sqrt(),
            false => 0.0,
        };
    }

    pub fn conjugate(&self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    fn normalize(&mut self) {
        let length = (self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w).sqrt();
        let n = 1.0 / length;
        self.x = n * self.x;
        self.y = n * self.y;
        self.z = n * self.z;
        self.w = n * self.w;
    }
}

impl NewableWithNameMap for FQuat {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

impl Newable for FQuat {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_f32::<LittleEndian>()?,
            y: reader.read_f32::<LittleEndian>()?,
            z: reader.read_f32::<LittleEndian>()?,
            w: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FVector {
    x: f32,
    y: f32,
    z: f32,
}

impl FVector {
    pub fn get_tuple(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }

    fn unit() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn unit_scale() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }
}

impl Newable for FVector {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_f32::<LittleEndian>()?,
            y: reader.read_f32::<LittleEndian>()?,
            z: reader.read_f32::<LittleEndian>()?,
        })
    }
}

impl NewableWithNameMap for FVector {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

#[derive(Debug, Serialize)]
pub struct FVector4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

impl FVector4 {
    pub fn get_tuple(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.z, self.w)
    }

    pub fn get_tuple3(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }

    pub fn get_normal(&self) -> Self {
        let length = ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt();
        if length == 0.0f32 { // literally no idea wtf to do here
            return Self {
                x: 0.0f32,
                y: 0.0f32,
                z: 1.0f32,
                w: 1.0f32,
            };
        }
        Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
            w: match self.w > 0.0f32 {
                true => -1.0f32,
                false => 1.0f32,
            },
        }
    }
}

impl Newable for FVector4 {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            x: reader.read_f32::<LittleEndian>()?,
            y: reader.read_f32::<LittleEndian>()?,
            z: reader.read_f32::<LittleEndian>()?,
            w: reader.read_f32::<LittleEndian>()?,
        })
    }
}

impl NewableWithNameMap for FVector4 {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

#[derive(Debug, Serialize)]
struct FBox {
    min: FVector,
    max: FVector,
    valid: bool,
}

impl Newable for FBox {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            min: FVector::new(reader)?,
            max: FVector::new(reader)?,
            valid: reader.read_u32::<LittleEndian>()? != 0,
        })
    }
}

impl NewableWithNameMap for FBox {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

#[derive(Debug, Serialize)]
struct FBox2D {
    min: FVector2D,
    max: FVector2D,
    valid: bool,
}

impl Newable for FBox2D {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            min: FVector2D::new(reader)?,
            max: FVector2D::new(reader)?,
            valid: reader.read_u32::<LittleEndian>()? != 0,
        })
    }
}

impl NewableWithNameMap for FBox2D {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Self::new(reader)
    }
}

#[derive(Debug, Serialize)]
struct FRotator {
    pitch: f32,
    yaw: f32,
    roll: f32,
}

impl NewableWithNameMap for FRotator {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            pitch: reader.read_f32::<LittleEndian>()?,
            yaw: reader.read_f32::<LittleEndian>()?,
            roll: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FPerPlatformFloat {
    cooked: bool,
    value: f32,
}

impl NewableWithNameMap for FPerPlatformFloat {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            cooked: reader.read_u8()? != 0,
            value: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FPerPlatformInt {
    cooked: bool,
    value: u32,
}

impl NewableWithNameMap for FPerPlatformInt {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            cooked: reader.read_u8()? != 0,
            value: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FWeightedRandomSampler {
    prob: Vec<f32>,
    alias: Vec<i32>,
    total_weight: f32,
}

impl NewableWithNameMap for FWeightedRandomSampler {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            prob: read_tarray(reader)?,
            alias: read_tarray(reader)?,
            total_weight: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FRichCurveKey {
    interp_mode: u8,
    tangent_mode: u8,
    tangent_weight_mode: u8,
    time: f32,
    value: f32,
    arrive_tangent: f32,
    arrive_tangent_weight: f32,
    leave_tangent: f32,
    leave_tangent_weight: f32,
}

impl NewableWithNameMap for FRichCurveKey {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            interp_mode: reader.read_u8()?,
            tangent_mode: reader.read_u8()?,
            tangent_weight_mode: reader.read_u8()?,
            time: reader.read_f32::<LittleEndian>()?,
            value: reader.read_f32::<LittleEndian>()?,
            arrive_tangent: reader.read_f32::<LittleEndian>()?,
            arrive_tangent_weight: reader.read_f32::<LittleEndian>()?,
            leave_tangent: reader.read_f32::<LittleEndian>()?,
            leave_tangent_weight: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FCompressedOffsetData {
    offset_data: Vec<i32>,
    strip_size: i32,
}

#[derive(Debug, Serialize)]
struct FSmartName {
    display_name: String,
}

impl NewableWithNameMap for FSmartName {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            display_name: read_fname(reader, name_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FCompressedSegment {
    start_frame: i32,
    num_frames: i32,
    byte_stream_offset: i32,
    translation_compression_format: u8,
    rotation_compression_format: u8,
    scale_compression_format: u8,
}

impl Newable for FCompressedSegment {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            start_frame: reader.read_i32::<LittleEndian>()?,
            num_frames: reader.read_i32::<LittleEndian>()?,
            byte_stream_offset: reader.read_i32::<LittleEndian>()?,
            translation_compression_format: reader.read_u8()?,
            rotation_compression_format: reader.read_u8()?,
            scale_compression_format: reader.read_u8()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FSimpleCurveKey {
    time: f32,
    value: f32,
}

impl NewableWithNameMap for FSimpleCurveKey {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            time: reader.read_f32::<LittleEndian>()?,
            value: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FDateTime {
    date: i64,
}

impl NewableWithNameMap for FDateTime {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            date: reader.read_i64::<LittleEndian>()?,
        })
    }
}

// I have no idea how this works
#[derive(Debug, Serialize)]
pub struct FScriptDelegate {
    object: i32,
    name: String,
}

impl NewableWithNameMap for FScriptDelegate {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            object: reader.read_i32::<LittleEndian>()?,
            name: read_fname(reader, name_map)?,
        })
    }
}

impl UScriptStruct {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, struct_name: &str) -> ParserResult<Self> {
        let err = |v| ParserError::add(v, format!("Struct Type: {}", struct_name));
        let struct_type: Box<dyn NewableWithNameMap> = match struct_name {
            "LinearColor" => Box::new(FLinearColor::new_n(reader, name_map, import_map).map_err(err)?),
            "Color" => Box::new(FColor::new_n(reader, name_map, import_map).map_err(err)?),
            "GameplayTagContainer" => Box::new(FGameplayTagContainer::new_n(reader, name_map, import_map).map_err(err)?),
            "IntPoint" => Box::new(FIntPoint::new_n(reader, name_map, import_map).map_err(err)?),
            "Guid" => Box::new(FGuid::new(reader).map_err(err)?),
            "Quat" => Box::new(FQuat::new_n(reader, name_map, import_map).map_err(err)?),
            "Vector" => Box::new(FVector::new_n(reader, name_map, import_map).map_err(err)?),
            "Vector2D" => Box::new(FVector2D::new_n(reader, name_map, import_map).map_err(err)?),
            "Rotator" => Box::new(FRotator::new_n(reader, name_map, import_map).map_err(err)?),
            "Box" => Box::new(FBox::new_n(reader, name_map, import_map).map_err(err)?),
            "Box2D" => Box::new(FBox2D::new_n(reader, name_map, import_map).map_err(err)?),
            "PerPlatformFloat" => Box::new(FPerPlatformFloat::new_n(reader, name_map, import_map).map_err(err)?),
            "PerPlatformInt" => Box::new(FPerPlatformInt::new_n(reader, name_map, import_map).map_err(err)?),
            "SkeletalMeshSamplingLODBuiltData" => Box::new(FWeightedRandomSampler::new_n(reader, name_map, import_map).map_err(err)?),
            "SoftObjectPath" => Box::new(FSoftObjectPath::new_n(reader, name_map, import_map).map_err(err)?),
            "SoftClassPath" => Box::new(FSoftObjectPath::new_n(reader, name_map, import_map).map_err(err)?),
            "LevelSequenceObjectReferenceMap" => Box::new(FLevelSequenceObjectReferenceMap::new_n(reader, name_map, import_map).map_err(err)?),
            "FrameNumber" => Box::new(FI32::new_n(reader, name_map, import_map).map_err(err)?),
            "SectionEvaluationDataTree" => Box::new(FSectionEvaluationDataTree::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneTrackIdentifier" => Box::new(FI32::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneSegment" => Box::new(FMovieSceneSegment::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneEvalTemplatePtr" => Box::new(InlineUStruct::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneTrackImplementationPtr" => Box::new(InlineUStruct::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneSequenceInstanceDataPtr" => Box::new(InlineUStruct::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneFrameRange" => Box::new(FMovieSceneFrameRange::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneSegmentIdentifier" => Box::new(FI32::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneSequenceID" => Box::new(FU32::new_n(reader, name_map, import_map).map_err(err)?),
            "MovieSceneEvaluationKey" => Box::new(FMovieSceneEvaluationKey::new_n(reader, name_map, import_map).map_err(err)?),
            "SmartName" => Box::new(FSmartName::new_n(reader, name_map, import_map).map_err(err)?),
            "RichCurveKey" => Box::new(FRichCurveKey::new_n(reader, name_map, import_map).map_err(err)?),
            "SimpleCurveKey" => Box::new(FSimpleCurveKey::new_n(reader, name_map, import_map).map_err(err)?),
            "DateTime" => Box::new(FDateTime::new_n(reader, name_map, import_map).map_err(err)?),
            "Timespan" => Box::new(FDateTime::new_n(reader, name_map, import_map).map_err(err)?),
            _ => Box::new(FStructFallback::new_unversioned(reader, name_map, import_map, struct_name).map_err(err)?),
        };
        Ok(Self {
            struct_name: struct_name.to_owned(),
            struct_type: struct_type,
        })
    }

    pub fn get_contents(&self) -> &Vec<FPropertyTag> {
        &self.struct_type.get_properties().unwrap()
    }
}

impl Serialize for UScriptStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.struct_type.serialize(serializer)
    }
}

#[derive(Debug)]
pub struct UScriptArray {
    tag: Option<Box<FPropertyTag>>,
    data: Vec<FPropertyTagType>,
}

impl UScriptArray {
    fn new_unversioned(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, mapping: &TagMapping) -> ParserResult<Self> {
        let element_count = reader.read_u32::<LittleEndian>()?;

        let mut data = Vec::new();
        for i in 0..element_count {
            let cpos = reader.position();
            let err = |v| ParserError::add(v, format!("Array Item: {} of {} at {}", i, element_count, cpos));
            data.push(read_unversioned_tag(reader, name_map, import_map, mapping).map_err(err)?);
        }
        
        Ok(Self {
            tag: None,
            data,
        })
    }

    pub fn get_data(&self) -> &Vec<FPropertyTagType> {
        &self.data
    }
}

impl Serialize for UScriptArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.data.len()))?;
        for e in &self.data {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

#[derive(Debug)]
pub struct UScriptMap {
    map_data: Vec<(FPropertyTagType, FPropertyTagType)>,
}

impl UScriptMap {
    fn new_unversioned(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, key_type: &TagMapping, value_type: &TagMapping) -> ParserResult<Self> {
        let remove_keys = reader.read_u32::<LittleEndian>()?;
        let element_count = reader.read_u32::<LittleEndian>()?;
        let rpos = reader.position();

        let f_key_type = match key_type {
            TagMapping::EnumProperty { .. } => TagMapping::NameProperty,
            _ => key_type.clone(),
        };

        let mut map_data = Vec::new();
        for i in 0..element_count {
            let err_f = |v| ParserError::add(v, format!("MapProperty error, types: {} of {} {} {:#?} {:#?} {}", i, element_count, remove_keys, key_type, value_type, rpos));
            map_data.push((
                read_unversioned_tag(reader, name_map, import_map, &f_key_type).map_err(err_f)?,
                read_unversioned_tag(reader, name_map, import_map, value_type).map_err(err_f)?
            ));
        }

        Ok(Self {
            map_data
        })
    }
}

struct TempSerializeTuple<'a, K, V> {
    key: &'a K,
    value: &'a V,
}

impl<'a, K,V> Serialize for TempSerializeTuple<'a, K, V>
where
    K: Serialize,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("key", self.key)?;
        map.serialize_entry("value", self.value)?;
        map.end()
    }
}

impl Serialize for UScriptMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.map_data.len()))?;
        for e in &self.map_data {
            let obj = TempSerializeTuple {
                key: &e.0,
                value: &e.1,
            };
            seq.serialize_element(&obj)?;
        }
        seq.end()
    }
}

#[derive(Debug, Serialize)]
pub struct UInterfaceProperty {
    interface_number: u32,
}

impl NewableWithNameMap for UInterfaceProperty {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            interface_number: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct FFieldPath {
    names: Vec<String>,
}

impl NewableWithNameMap for FFieldPath {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let names: Vec<String> = read_tarray_n(reader, name_map, import_map)?;
        Ok(Self {
            names,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FPropertyTagType {
    BoolProperty(bool),
    StructProperty(UScriptStruct),
    ObjectProperty(FPackageIndex),
    InterfaceProperty(UInterfaceProperty),
    DelegateProperty(FScriptDelegate),
    MulticastDelegateProperty(Vec<FScriptDelegate>),
    FloatProperty(f32),
    TextProperty(FText),
    StrProperty(String),
    NameProperty(String),
    IntProperty(i32),
    UInt16Property(u16),
    UInt32Property(u32),
    UInt64Property(u64),
    ArrayProperty(UScriptArray),
    MapProperty(UScriptMap),
    ByteProperty(u8),
    EnumProperty(Option<String>),
    SoftObjectProperty(FSoftObjectPath),
    SoftObjectPropertyMap(FGuid),
    FieldPathProperty(FFieldPath),
}

#[derive(Debug)]
pub struct FPropertyTag {
    name: String,
    size: i32,
    tag: FPropertyTagType,
}

impl FPropertyTag {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_data(&self) -> &FPropertyTagType {
        &self.tag
    }
}

fn read_unversioned_tag(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, mapping: &TagMapping) -> ParserResult<FPropertyTagType> {
    Ok(match mapping {
        TagMapping::TextProperty => FPropertyTagType::TextProperty(FText::new(reader)?),
        TagMapping::StrProperty => FPropertyTagType::StrProperty(read_string(reader)?),
        TagMapping::NameProperty => FPropertyTagType::NameProperty(read_fname(reader, name_map)?),
        TagMapping::StructProperty { struct_type } => FPropertyTagType::StructProperty(UScriptStruct::new(reader, name_map, import_map, struct_type)?),
        TagMapping::ObjectProperty => FPropertyTagType::ObjectProperty(FPackageIndex::new_n(reader, name_map, import_map)?),
        TagMapping::SoftObjectProperty => FPropertyTagType::SoftObjectProperty(FSoftObjectPath::new_n(reader, name_map, import_map)?),
        TagMapping::EnumProperty { enum_name } => {
            let val = reader.read_u8()?;
            let data = match MAPPINGS.get_enum_mapping(enum_name, val as usize) {
                Some(d) => d.to_owned(),
                None => val.to_string(),
            };

            FPropertyTagType::EnumProperty(Some(data))
        },
        TagMapping::ArrayProperty { inner_type } => FPropertyTagType::ArrayProperty(UScriptArray::new_unversioned(reader, name_map, import_map, inner_type)?),
        TagMapping::MapProperty { inner_type, value_type } => FPropertyTagType::MapProperty(UScriptMap::new_unversioned(reader, name_map, import_map, inner_type, value_type)?),
        TagMapping::BoolProperty => FPropertyTagType::BoolProperty(reader.read_u8()? != 0),
        TagMapping::ByteProperty => FPropertyTagType::ByteProperty(reader.read_u8()?),
        TagMapping::IntProperty => FPropertyTagType::IntProperty(reader.read_i32::<LittleEndian>()?),
        TagMapping::FloatProperty => FPropertyTagType::FloatProperty(reader.read_f32::<LittleEndian>()?),
        TagMapping::DebugProperty => return Err(ParserError::new(format!("Encountered DebugProperty - Stopping"))),
        _ => return Err(ParserError::new(format!("Unsupported Property Type: {:#?}", mapping))),
    })
}

fn read_unversioned_property(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, mapping: &PropertyMapping) -> ParserResult<FPropertyTag> {
    let start_pos = reader.position();

    let err = |v| ParserError::add(v, format!("Property: {} at {}", mapping.get_name(), start_pos));
    let tag = read_unversioned_tag(reader, name_map, import_map, &mapping.get_type()).map_err(err)?;

    // println!("Tag: {} {} {:#?}", start_pos, mapping.get_name(), tag);

    let size = (reader.position() - start_pos) as i32;
    Ok(FPropertyTag {
        name: mapping.get_name().to_owned(),
        size,
        tag,
    })
}

#[derive(Debug)]
struct FStripDataFlags {
    global_strip_flags: u8,
    class_strip_flags: u8,
}

impl FStripDataFlags {
    fn is_editor_data_stripped(&self) -> bool {
        (self.global_strip_flags & 1) != 0
    }

    fn is_data_stripped_for_server(&self) -> bool {
        (self.global_strip_flags & 2) != 0
    }

    fn is_class_data_stripped(&self, flag: u8) -> bool {
        (self.class_strip_flags & flag) != 0
    }
}

impl Newable for FStripDataFlags {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            global_strip_flags: reader.read_u8()?,
            class_strip_flags: reader.read_u8()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FByteBulkDataHeader {
    bulk_data_flags: i32,
    element_count: i32,
    size_on_disk: i32,
    offset_in_file: i64,
}

impl Newable for FByteBulkDataHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            bulk_data_flags: reader.read_i32::<LittleEndian>()?,
            element_count: reader.read_i32::<LittleEndian>()?,
            size_on_disk: reader.read_i32::<LittleEndian>()?,
            offset_in_file: reader.read_i64::<LittleEndian>()?,
        })
    }
}

struct FByteBulkData {
    header: FByteBulkDataHeader,
    data: Vec<u8>
}

impl std::fmt::Debug for FByteBulkData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Header: {:?} {}", self.header, self.data.len())
    }
}

impl Serialize for FByteBulkData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.header.serialize(serializer)
    }
}

impl FByteBulkData {
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>) -> ParserResult<Self> {
        let header = FByteBulkDataHeader::new(reader)?;
        let mut data: Vec<u8> = Vec::new();

        if header.bulk_data_flags & 0x0040 != 0 {
            data.resize(header.element_count as usize, 0u8);
            reader.read_exact(&mut data)?;
        }

        if header.bulk_data_flags & 0x0100 != 0 {
            let ubulk_reader = match ubulk {
                Some(data) => data,
                None => return Err(ParserError::new(format!("No ubulk specified for texture"))),
            };

            // If the header bulk data is for the uptnl, but the buffer isn't long enough, it's *probably* the ubulk.
            if !(header.bulk_data_flags & (1 << 11) != 0 && ubulk_reader.get_mut().len() < header.element_count as usize) {
                let offset = header.offset_in_file;
                data.resize(header.element_count as usize, 0u8);
                ubulk_reader.seek(SeekFrom::Start(offset as u64)).unwrap();
                ubulk_reader.read_exact(&mut data).unwrap();
            }
        }

        Ok(Self {
            header, data
        })
    }
}

#[derive(Debug)]
pub struct FTexture2DMipMap {
    data: FByteBulkData,
    size_x: i32,
    size_y: i32,
    size_z: i32,
}

impl FTexture2DMipMap {
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>) -> ParserResult<Self> {
        let cooked = reader.read_i32::<LittleEndian>()?;
        let data = FByteBulkData::new(reader, ubulk)?;
        let size_x = reader.read_i32::<LittleEndian>()?;
        let size_y = reader.read_i32::<LittleEndian>()?;
        let size_z = reader.read_i32::<LittleEndian>()?;
        if cooked != 1 {
            read_string(reader)?;
        }

        Ok(Self {
            data, size_x, size_y, size_z
        })
    }
}

#[allow(dead_code)]
impl FTexture2DMipMap {
    pub fn get_bytes(&self) -> &Vec<u8> {
        &self.data.data
    }

    pub fn get_bytes_move(self) -> Vec<u8> {
        self.data.data
    }

    pub fn get_width(&self) -> u32 {
        self.size_x as u32
    }

    pub fn get_height(&self) -> u32 {
        self.size_y as u32
    }
}

#[derive(Debug)]
pub struct FTexturePlatformData {
    size_x: i32,
    size_y: i32,
    num_slices: i32,
    pixel_format: String,
    first_mip: i32,
    mips: Vec<FTexture2DMipMap>,
    is_virtual: bool,
}

impl FTexturePlatformData {
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>) -> ParserResult<Self> {
        let size_x = reader.read_i32::<LittleEndian>()?;
        let size_y = reader.read_i32::<LittleEndian>()?;
        let num_slices = reader.read_i32::<LittleEndian>()?;
        let pixel_format = read_string(reader)?;
        let first_mip = reader.read_i32::<LittleEndian>()?;
        let length = reader.read_u32::<LittleEndian>()?;
        let mut mips = Vec::new();
        for _i in 0..length {
            let mip = FTexture2DMipMap::new(reader, ubulk)?;
            if mip.data.data.len() <= 0 { continue; }
            mips.push(mip);
        }

        let is_virtual = reader.read_u32::<LittleEndian>()? != 0;
        if is_virtual {
            return Err(ParserError::new(format!("Texture is virtual, unsupported for now")));
        }

        Ok(Self {
            size_x, size_y, num_slices, pixel_format, first_mip, mips, is_virtual
        })
    }
}

pub trait PackageExport: std::fmt::Debug + TraitSerialize {
    fn get_export_type(&self) -> &str;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

serialize_trait_object!(PackageExport);

#[derive(Debug, Serialize)]
struct EmptyPackage {

}

impl PackageExport for EmptyPackage {
    fn get_export_type(&self) -> &str {
        "EmptyPackage"
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl EmptyPackage {
    fn new() -> Self {
        Self {

        }
    }
}

#[derive(Debug)]
struct FFragment {
    skip_num: u32,
    has_zeroes: bool,
    value: u32,
    is_last: bool,
}

impl Newable for FFragment {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let data = reader.read_u16::<LittleEndian>()? as u32;
        let skip_num = data & 0x007f;
        let has_zeroes = (data & 0x0080) != 0;
        let value = data >> 9;
        let is_last = (data & 0x0100) != 0;
        Ok(Self {
            skip_num, has_zeroes, value, is_last,
        })
    }
}

fn divide_round_up(dividend: u32, divisor: u32) -> u32 {
    (dividend + divisor - 1) / divisor
}

#[derive(Debug)]
struct PropertyIndex {
    index: u32,
    zero: bool,
}

#[derive(Debug)]
struct FUnversionedHeader {
    fragments: Vec<FFragment>,
    zero_data: BitVec,
}

impl Newable for FUnversionedHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut fragments = Vec::new();
        let mut zero_mask_num = 0;

        /*if reader.position() == 0 {
            return Ok(Self {
                fragments: Vec::new(),
                zero_data: BitVec::new(),
            });
        }*/

        loop {
            let property_header = FFragment::new(reader)?;
            let is_last = property_header.is_last;
            if property_header.has_zeroes {
                zero_mask_num += property_header.value;
            }
            fragments.push(property_header);

            if is_last { break; }
        }

        let zero_data = if zero_mask_num > 0 {
            let byte_count = match zero_mask_num {
                0..=8 => 1,
                9..=16 => 2,
                _ => divide_round_up(zero_mask_num, 32) * 4,
            };
            let mut bytes = vec![0u8; byte_count as usize];
            reader.read_exact(&mut bytes)?;
            BitVec::from_bytes(&bytes)
        } else {
            BitVec::new()
        };

        Ok(Self {
            fragments,
            zero_data,
        })
    }
}

impl FUnversionedHeader {
    fn is_zero(&self, idx: usize) -> bool {
        let t_byte = idx / 8;
        let t_bit = idx % 8;

        let target = (t_byte * 8) + (7 - t_bit);

        self.zero_data.get(target).unwrap()
    }

    fn get_indices(&self) -> Vec<PropertyIndex> {
        let mut i = 0;
        let mut zero_i = 0;
        let mut vals = Vec::new();
        for header in &self.fragments {
            i += header.skip_num;
            for t in 0..header.value {
                vals.push(PropertyIndex {
                    index: t + i,
                    zero: header.has_zeroes && self.is_zero(zero_i),
                });
                if header.has_zeroes { zero_i += 1; }
            }
            i += header.value;
        }

        vals
    }
}

#[derive(Debug)]
struct FExportBundleHeader {
    first_export: u32,
    export_count: u32,
}

impl Newable for FExportBundleHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            first_export: reader.read_u32::<LittleEndian>()?,
            export_count: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FExportBundleEntry {
    export_index: u32,
    command_type: u32,
}

impl Newable for FExportBundleEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            export_index: reader.read_u32::<LittleEndian>()?,
            command_type: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FExportBundle {
    header: FExportBundleHeader,
    entries: Vec<FExportBundleEntry>,
}

impl FExportBundle {
    fn get_export_order(&self) -> Vec<u32> {
        self.entries.iter().filter(|v| v.command_type == 1).map(|v| std::cmp::min(self.header.export_count - 1, v.export_index)).collect()
    }
}

impl Newable for FExportBundle {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let header = FExportBundleHeader::new(reader)?;
        let mut entries = Vec::new();
        for _i in 0..header.export_count {
            entries.push(FExportBundleEntry::new(reader)?);
        }

        Ok(Self {
            header,
            entries
        })
    }
}

#[derive(Debug)]
struct FArc {
    from_index: u32,
    to_index: u32,
}

impl Newable for FArc {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            from_index: reader.read_u32::<LittleEndian>()?,
            to_index: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FImportedPackage {
    index: u64,
    arcs: Vec<FArc>,
}

impl Newable for FImportedPackage {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            index: reader.read_u64::<LittleEndian>()?,
            arcs: read_tarray(reader)?,
        })
    }
}

impl Serialize for FImportedPackage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.index.to_string())
    }
}

/// A UObject is a struct for all of the parsed properties of an object
#[derive(Debug)]
pub struct UObject {
    export_type: String,
    export_index: Option<FPackageObjectIndex>,
    properties: Vec<FPropertyTag>,
}

impl UObject {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, export_type: &str, export_index: Option<FPackageObjectIndex>) -> ParserResult<Self> {
        let header = FUnversionedHeader::new(reader)?;

        let prop_indices = header.get_indices();
        let indices = prop_indices.iter().map(|v| v.index).collect();

        let mappings = MAPPINGS.get_mappings(export_type, indices)?;

        let mut properties = Vec::new();
        for i in 0..prop_indices.len() {
            let index = &prop_indices[i];
            let mapping = &mappings[i];
            if index.zero {
                let null_data = vec![0u8; 32];
                let mut cursor = Cursor::new(null_data.as_slice());
                properties.push(read_unversioned_property(&mut cursor, name_map, import_map, mapping)?);
            } else {
                properties.push(read_unversioned_property(reader, name_map, import_map, mapping)?);
            }
        }
        
        Ok(Self {
            properties,
            export_index,
            export_type: export_type.to_owned(),
        })
    }

    pub fn get_properties(&self) -> &Vec<FPropertyTag> {
        &self.properties
    }

    pub fn get_property(&self, name: &str) -> Option<&FPropertyTagType> {
        self.properties.iter().fold(None, |acc, v| {
            if v.get_name() == name {
                return Some(v.get_data());
            }
            acc
        })
    }

    pub fn get_boolean(&self, name: &str) -> Option<bool> {
        match self.get_property(name) {
            Some(FPropertyTagType::BoolProperty(bool_property)) => Some(*bool_property),
            _ => None,
        }
    }
}

impl Serialize for UObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.properties.len() + 2))?;
        map.serialize_entry("export_type", &self.export_type)?;
        map.serialize_entry("export_index", &self.export_index)?;
        for property in &self.properties {
            map.serialize_entry(&property.name, &property.tag)?;
        }
        map.end()
    }
}

impl PackageExport for UObject {
    fn get_export_type(&self) -> &str {
        &self.export_type
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Texture2D contains the details, parameters and mipmaps for a texture
#[derive(Debug)]
pub struct Texture2D {
    base_object: UObject,
    cooked: u32,
    textures: Vec<FTexturePlatformData>,
}

#[allow(dead_code)]
impl Texture2D {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, ubulk: &mut Option<ReaderCursor>, export_index: Option<FPackageObjectIndex>) -> ParserResult<Self> {
        let object = UObject::new(reader, name_map, import_map, "Texture2D", export_index)?;

        let _serialize_guid = reader.read_u32::<LittleEndian>()?;

        FStripDataFlags::new(reader)?; // still no idea
        FStripDataFlags::new(reader)?; // why there are two

        let mut textures: Vec<FTexturePlatformData> = Vec::new();
        let cooked = reader.read_u32::<LittleEndian>()?;
        if cooked == 1 {
            let mut pixel_format = read_fname(reader, name_map)?;
            while pixel_format != "None" {
                let _skip_offset = reader.read_i64::<LittleEndian>()?;
                let texture = FTexturePlatformData::new(reader, ubulk)?;
                // Seems to be always wrong, can't work out what it's referring to.
                /*if reader.position() != skip_offset as u64 {
                    panic!("Texture read incorrectly {} {}", reader.position(), skip_offset);
                }*/
                textures.push(texture);
                pixel_format = read_fname(reader, name_map)?;
            }
        }

        Ok(Self {
            base_object: object,
            cooked: cooked,
            textures: textures,
        })
    }

    pub fn get_pixel_format(&self) -> ParserResult<&str> {
        let pdata = match self.textures.get(0) {
            Some(data) => data,
            None => return Err(ParserError::new(format!("No textures found"))),
        };
        Ok(&pdata.pixel_format)
    }

    pub fn get_texture(&self) -> ParserResult<&FTexture2DMipMap> {
        let pdata = match self.textures.get(0) {
            Some(data) => data,
            None => return Err(ParserError::new(format!("No textures part of export"))),
        };
        let texture = match pdata.mips.get(0) {
            Some(data) => data,
            None => return Err(ParserError::new(format!("No mipmaps part of texture"))),
        };
        Ok(texture)
    }

    pub fn get_texture_move(mut self) -> ParserResult<FTexture2DMipMap> {
        if self.textures.len() <= 0 {
            return Err(ParserError::new(format!("No textures part of export")));
        }
        let mut texture = self.textures.swap_remove(0);
        if texture.mips.len() <= 0 {
            return Err(ParserError::new(format!("No mipmaps part of texture")));
        }
        Ok(texture.mips.swap_remove(0))
    }
}

impl PackageExport for Texture2D {
    fn get_export_type(&self) -> &str {
        "Texture2D"
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Serialize for Texture2D {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.base_object.serialize(serializer)
    }
}

#[derive(Debug)]
pub struct UDataTable {
    super_object: UObject,
    rows: Vec<(String, UObject)>,
}

impl PackageExport for UDataTable {
    fn get_export_type(&self) -> &str {
        "DataTable"
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl UDataTable {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "DataTable", None)?;

        // Find the RowStruct type
        let row_prop = match super_object.properties.iter().find(|v| v.name == "RowStruct") {
            Some(p) => p,
            None => return Err(ParserError::new(format!("RowStruct not found"))),
        };
        let package_index = match row_prop.get_data() {
            FPropertyTagType::ObjectProperty(index) => index,
            _ => return Err(ParserError::new(format!("RowStruct is not ObjectProperty"))),
        };
        let object_index = match package_index.get_import() {
            Some(import) => import,
            None => return Err(ParserError::new(format!("Import not found in Import Map"))),
        };
        let struct_name = object_index.get_export_name(name_map, import_map)?;

        let _zero_data = reader.read_i32::<LittleEndian>()?;
        let num_rows = reader.read_i32::<LittleEndian>()?;

        let mut rows = Vec::new();

        for _i in 0..num_rows {
            let row_name = read_fname(reader, name_map)?;
            let row_object = UObject::new(reader, name_map, import_map, &struct_name, None)?;
            rows.push((row_name, row_object));
        }

        Ok(Self {
            super_object, rows,
        })
    }
}

impl Serialize for UDataTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some((self.rows.len() + 1) as usize))?;
        map.serialize_entry("export_type", "DataTable")?;
        for e in &self.rows {
            map.serialize_entry(&e.0, &e.1)?;
        }
        map.end()
    }
}

/*#[derive(Debug, Serialize)]
enum ECurveTableMode {
    Empty,
    SimpleCurves,
    RichCurves,
}

#[derive(Debug, Serialize)]
struct UCurveTable {
    super_object: UObject,
    curve_table_mode: ECurveTableMode,
    row_map: Vec<(String, UObject)>,
}

impl UCurveTable {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "CurveTable")?;
        let num_rows = reader.read_i32::<LittleEndian>()?;
        let curve_table_mode = reader.read_u8()?;
        let curve_table_mode = match curve_table_mode {
            0 => ECurveTableMode::Empty,
            1 => ECurveTableMode::SimpleCurves,
            2 => ECurveTableMode::RichCurves,
            _ => panic!("unsupported curve mode"),
        };

        let mut row_map = Vec::new();
        for _i in 0..num_rows {
            let row_name = read_fname(reader, name_map)?;
            let row_type = match curve_table_mode {
                ECurveTableMode::Empty => "Empty",
                ECurveTableMode::SimpleCurves => "SimpleCurveKey",
                ECurveTableMode::RichCurves => "RichCurveKey",
            }.to_owned();
            let row_curve = UObject {
                properties: UObject::serialize_properties(reader, name_map, import_map)?,
                export_type: row_type.to_owned(),
            };
            row_map.push((row_name, row_curve));
        }

        Ok(Self {
            super_object, curve_table_mode, row_map,
        })
    }
}

impl PackageExport for UCurveTable {
    fn get_export_type(&self) -> &str {
        "CurveTable"
    }
}*/

fn select_export(export_name: &str, reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, export: &FExportMapEntry, ubulk: &mut Option<ReaderCursor>) -> ParserResult<Box<dyn PackageExport>> {
    let export_index = Some(export.global_import_index.clone());
    Ok(match export_name {
        "Texture2D" => Box::new(Texture2D::new(reader, name_map, import_map, ubulk, export_index)?),
        "DataTable" => Box::new(UDataTable::new(reader, name_map, import_map)?),
        _ => Box::new(UObject::new(reader, name_map, import_map, export_name, export_index)?),
    })
}

/// A Package is the collection of parsed data from a uasset/uexp file combo
///
/// It contains a number of 'Exports' which could be of any type implementing the `PackageExport` trait
/// Note that exports are of type `dyn Any` and will need to be downcasted to their appropriate types before being usable
pub struct Package {
    summary: FPackageSummary,
    exports: Vec<Box<dyn PackageExport>>,
    graph_data: Vec<FImportedPackage>,
}

#[allow(dead_code)]
impl Package {
    pub fn from_buffer(uasset: &[u8], ubulk: Option<&[u8]>, global_map: &LoaderGlobalData) -> ParserResult<Self> {
        let mut cursor = ReaderCursor::new(uasset);
        let summary = FPackageSummary::new(&mut cursor)?;

        let mut name_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.name_map_offset as u64))?;
        
        while cursor.position() <= (summary.name_map_offset + summary.name_map_size) as u64 {
            name_map.push(read_short_string(&mut cursor)?);
        }

        let name_map = FNameMap::from_strings(name_map);

        cursor.seek(SeekFrom::Start(summary.import_map_offset as u64))?;
        let import_length = (summary.export_map_offset - summary.import_map_offset) / 8;
        let mut import_map = Vec::new();
        for _i in 0..import_length {
            import_map.push(FPackageObjectIndex::new(&mut cursor)?);
        }
        
        let mut export_map = Vec::new();
        while cursor.position() < summary.export_bundle_offset as u64 {
            export_map.push(FExportMapEntry::new(&mut cursor)?);
        }

        let export_bundle = FExportBundle::new(&mut cursor)?;
        let export_order = export_bundle.get_export_order();

        cursor.seek(SeekFrom::Start(summary.graph_data_offset as u64))?;
        let graph_data: Vec<FImportedPackage> = read_tarray(&mut cursor)?;

        cursor.seek(SeekFrom::Start((summary.graph_data_offset + summary.graph_data_size) as u64))?;

        let mut ubulk_cursor = match ubulk {
            Some(data) => Some(ReaderCursor::new(data)),
            None => None,
        };

        let mut exports: Vec<Box<dyn PackageExport>> = Vec::new();
        for _i in 0..export_map.len() {
            exports.push(Box::new(EmptyPackage::new()));
        }

        let import_map = ImportMap {
            imports: import_map,
            global: global_map.get_load_data(),
        };

        let name_map = NameMap {
            names: name_map,
            global: global_map.get_name_map(),
        };

        let mut export_start = cursor.position();

        for export_idx in &export_order {
            let export = &export_map[*export_idx as usize];
            let export_name = export.get_export_name(&name_map, &import_map)?;
            let mut export_data = select_export(&export_name, &mut cursor, &name_map, &import_map, &export, &mut ubulk_cursor)
                .map_err(|v| ParserError::add(v, format!("Export Type: {}", export_name)))?;
            std::mem::swap(&mut export_data, &mut exports[*export_idx as usize]);

            export_start += export.serial_size;
            cursor.seek(SeekFrom::Start(export_start))?;
        }

        Ok(Self {
            summary: summary,
            exports: exports,
            graph_data: graph_data,
        })
    }

    pub fn from_file(file_path: &str, global_map: &LoaderGlobalData) -> ParserResult<Self> {
        let asset_file = file_path.to_owned() + ".uasset";
        let ubulk_file = file_path.to_owned() + ".ubulk";

        // read asset file
        let mut asset = File::open(asset_file).map_err(|_v| ParserError::new(format!("Could not find file: {}", file_path)))?;
        let mut uasset_buf = Vec::new();
        asset.read_to_end(&mut uasset_buf)?;

        // read ubulk file (if exists)
        let ubulk_path = Path::new(&ubulk_file);
        let ubulk_buf = match metadata(ubulk_path).is_ok() {
            true => {
                let mut ubulk = File::open(ubulk_file)?;
                let mut ubulk_ibuf = Vec::new();
                ubulk.read_to_end(&mut ubulk_ibuf)?;
                Some(ubulk_ibuf)
            },
            false => None,
        };

        // this is some real wtfery
        let uptnl_file = file_path.to_owned() + ".uptnl";
        let uptnl_path = Path::new(&uptnl_file);
        let uptnl_buf = match metadata(uptnl_path).is_ok() {
            true => {
                let mut uptnl = File::open(uptnl_file)?;
                let mut uptnl_ibuf = Vec::new();
                uptnl.read_to_end(&mut uptnl_ibuf)?;
                Some(uptnl_ibuf)
            },
            false => None,
        };

        let ubulk_buf = match uptnl_buf {
            Some(b) => Some(b),
            None => ubulk_buf,
        };

        // ??
        match ubulk_buf {
            Some(data) => Self::from_buffer(&uasset_buf, Some(&data), global_map),
            None => Self::from_buffer(&uasset_buf, None, global_map),
        }
    }

    pub fn get_exports(self) -> Vec<Box<dyn PackageExport>> {
        self.exports
    }

    /// Returns a reference to an export
    ///
    /// Export will live as long as the underlying Package
    pub fn get_export(&self, index: usize) -> ParserResult<&dyn PackageExport> {
        Ok(match self.exports.get(index) {
            Some(data) => data,
            None => return Err(ParserError::new(format!("index {} out of range", index))),
        }.as_ref())
    }

    pub fn get_export_move(mut self, index: usize) -> ParserResult<Box<dyn PackageExport>> {
        if index < self.exports.len() {
            Ok(self.exports.swap_remove(index))
        } else {
            Err(ParserError::new(format!("No exports found")))
        }
    }

    pub fn empty() -> Self {
        Self {
            summary: FPackageSummary::empty(),
            exports: Vec::new(),
            graph_data: Vec::new(),
        }
    }
}

impl fmt::Debug for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for export in &self.exports {
            write!(f, "Export: {:#?}\n", export)?
        }
        write!(f, "Package Summary: {:#?}\n", self.summary)
    }
}

impl Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("Package", 2)?;
        state.serialize_field("exports", &self.exports)?;
        state.serialize_field("imported_packages", &self.graph_data)?;
        state.end()
    }
}
