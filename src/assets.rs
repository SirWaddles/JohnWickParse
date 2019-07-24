use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::fs::{File, metadata};
use std::path::Path;
use std::any::Any;
use half::f16;
use serde::ser::{Serialize, Serializer, SerializeMap, SerializeSeq, SerializeStruct};
use erased_serde::{Serialize as TraitSerialize};
use byteorder::{LittleEndian, ReadBytesExt};

pub mod locale;
mod material_instance;
mod anims;
mod meshes;

pub use anims::{USkeleton, UAnimSequence, FTrack};
pub use meshes::{USkeletalMesh, FMultisizeIndexContainer, FStaticMeshVertexDataTangent, FSkeletalMeshRenderData, 
    FSkelMeshRenderSection, FSkeletalMaterial, FSkinWeightVertexBuffer, FMeshBoneInfo, FStaticMeshVertexDataUV, FReferenceSkeleton};

pub type ReaderCursor = Cursor<Vec<u8>>;

/// ParserError contains a list of error messages that wind down to where the parser was not able to parse a property
#[derive(Debug)]
pub struct ParserError {
    property_list: Vec<String>,
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
        }
    }

    pub fn add(mut error: ParserError, property: String) -> Self {
        error.property_list.push(property);
        error
    }

    pub fn get_properties(&self) -> &Vec<String> {
        &self.property_list
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

impl std::error::Error for ParserError { }

pub type ParserResult<T> = Result<T, ParserError>;

pub trait Newable {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> where Self: Sized;
}

#[derive(Debug, PartialEq)]
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

type NameMap = Vec<FNameEntrySerialized>;
type ImportMap = Vec<FObjectImport>;

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
    let index_pos = reader.position();
    let name_index = reader.read_i32::<LittleEndian>()?;
    reader.read_i32::<LittleEndian>()?; // name_number ?
    match name_map.get(name_index as usize) {
        Some(data) => Ok(data.data.to_owned()),
        None => Err(ParserError::new(format!("FName could not be read at {} {}", index_pos, name_index))),
    }
}

#[derive(Debug, Clone)]
pub struct FPackageIndex {
    index: i32,
    import: String,
}

impl FPackageIndex {
    fn get_package<'a>(index: i32, import_map: &'a ImportMap) -> Option<&'a FObjectImport> {
        if index < 0 {
            return import_map.get((index * -1 - 1) as usize);
        }
        if index > 0 {
            return import_map.get((index - 1) as usize);
        }
        None
    }

    pub fn get_import(&self) -> &str {
        &self.import
    }
}

impl NewableWithNameMap for FPackageIndex {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let import = match FPackageIndex::get_package(index, import_map) {
            Some(data) => data.object_name.clone(),
            None => index.to_string(),
        };
        Ok(Self {
            index,
            import,
        })
    }
}

impl Serialize for FPackageIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.import)
    }
}

#[derive(Debug)]
struct FObjectImport {
    class_package: String,
    class_name: String,
    outer_index: FPackageIndex,
    object_name: String,
}

impl NewableWithNameMap for FObjectImport {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            class_package: read_fname(reader, name_map)?,
            class_name: read_fname(reader, name_map)?,
            outer_index: FPackageIndex::new_n(reader, name_map, import_map)?,
            object_name: read_fname(reader, name_map)?,
        })
    }
}

impl Serialize for FObjectImport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.object_name)
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
pub struct FText {
    flags: u32,
    history_type: i8,
    namespace: String,
    key: String,
    source_string: String,
}

impl Newable for FText {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let flags = reader.read_u32::<LittleEndian>()?;
        let history_type = reader.read_i8()?;

        match history_type {
            -1 => Ok(Self {
                flags,
                history_type,
                namespace: "".to_owned(),
                key: "".to_owned(),
                source_string: "".to_owned(),
            }),
            0 => Ok(Self {
                flags,
                history_type,
                namespace: read_string(reader)?,
                key: read_string(reader)?,
                source_string: read_string(reader)?,
            }),
            _ => Err(ParserError::new(format!("Could not read history type: {}", history_type))),
        }        
    }
}

impl Serialize for FText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("FText", 2)?;
        state.serialize_field("string", &self.source_string)?;
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
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let mut properties = Vec::new();
        loop {
            let tag = read_property_tag(reader, name_map, import_map, true)?;
            let tag = match tag {
                Some(data) => data,
                None => break,
            };

            properties.push(tag);
        }
        
        Ok(Self {
            properties: properties,
        })
    }

    fn get_properties(&self) -> ParserResult<&Vec<FPropertyTag>> {
        Ok(&self.properties)
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
    struct_type: Box<NewableWithNameMap>,
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

impl UScriptStruct {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, struct_name: &str) -> ParserResult<Self> {
        let err = |v| ParserError::add(v, format!("Struct Type: {}", struct_name));
        let struct_type: Box<NewableWithNameMap> = match struct_name {
            "Vector2D" => Box::new(FVector2D::new_n(reader, name_map, import_map).map_err(err)?),
			"Box2D" => Box::new(FVector2D::new_n(reader, name_map, import_map).map_err(err)?),
            "LinearColor" => Box::new(FLinearColor::new_n(reader, name_map, import_map).map_err(err)?),
            "Color" => Box::new(FColor::new_n(reader, name_map, import_map).map_err(err)?),
            "GameplayTagContainer" => Box::new(FGameplayTagContainer::new_n(reader, name_map, import_map).map_err(err)?),
            "IntPoint" => Box::new(FIntPoint::new_n(reader, name_map, import_map).map_err(err)?),
            "Guid" => Box::new(FGuid::new(reader).map_err(err)?),
            "Quat" => Box::new(FQuat::new_n(reader, name_map, import_map).map_err(err)?),
            "Vector" => Box::new(FVector::new_n(reader, name_map, import_map).map_err(err)?),
            "Rotator" => Box::new(FRotator::new_n(reader, name_map, import_map).map_err(err)?),
            "PerPlatformFloat" => Box::new(FPerPlatformFloat::new_n(reader, name_map, import_map).map_err(err)?),
            "PerPlatformInt" => Box::new(FPerPlatformInt::new_n(reader, name_map, import_map).map_err(err)?),
            "SkeletalMeshSamplingLODBuiltData" => Box::new(FWeightedRandomSampler::new_n(reader, name_map, import_map).map_err(err)?),
            "SoftObjectPath" => Box::new(FSoftObjectPath::new_n(reader, name_map, import_map).map_err(err)?),
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
            _ => Box::new(FStructFallback::new_n(reader, name_map, import_map).map_err(err)?),
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
    fn new(reader: &mut ReaderCursor, inner_type: &str, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let element_count = reader.read_u32::<LittleEndian>()?;
        let mut array_tag: Option<Box<FPropertyTag>> = None;
        if inner_type == "StructProperty" || inner_type == "ArrayProperty" {
            array_tag = match read_property_tag(reader, name_map, import_map, false)? {
                Some(data) => Some(Box::new(data)),
                None => panic!("Could not read file"),
            };
        }
        let inner_tag_data = match &array_tag {
            Some(data) => Some(&data.tag_data),
            None => None,
        };

        let mut contents: Vec<FPropertyTagType> = Vec::new();
        for _i in 0..element_count {
            if inner_type == "BoolProperty" {
                contents.push(FPropertyTagType::BoolProperty(reader.read_u8()? != 0));
                continue;
            }
            if inner_type == "ByteProperty" {
                contents.push(FPropertyTagType::ByteProperty(reader.read_u8()?));
                continue;
            }
            contents.push(FPropertyTagType::new(reader, name_map, import_map, &inner_type, inner_tag_data)?);
        }

        Ok(Self {
            tag: array_tag,
            data: contents,
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

fn read_map_value(reader: &mut ReaderCursor, inner_type: &str, struct_type: &str, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<FPropertyTagType> {
    Ok(match inner_type {
        "BoolProperty" => FPropertyTagType::BoolProperty(reader.read_u8()? != 1),
        "ByteProperty" => FPropertyTagType::ByteProperty(reader.read_u8()?),
        "EnumProperty" => FPropertyTagType::EnumProperty(Some(read_fname(reader, name_map)?)),
        "UInt32Property" => FPropertyTagType::UInt32Property(reader.read_u32::<LittleEndian>()?),
        "StructProperty" => FPropertyTagType::StructProperty(UScriptStruct::new(reader, name_map, import_map, struct_type)?),
        "NameProperty" => FPropertyTagType::NameProperty(read_fname(reader, name_map)?),
        "ObjectProperty" => FPropertyTagType::ObjectProperty(FPackageIndex::new_n(reader, name_map, import_map)?),
		"StrProperty" => FPropertyTagType::StrProperty(read_string(reader)?),
		"TextProperty" => FPropertyTagType::TextProperty(FText::new(reader)?),
        _ => FPropertyTagType::StructProperty(UScriptStruct::new(reader, name_map, import_map, inner_type)?),
    })
}

impl UScriptMap {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, key_type: &str, value_type: &str) -> ParserResult<Self> {
        let num_keys_to_remove = reader.read_i32::<LittleEndian>()?;
        if num_keys_to_remove != 0 {
            return Err(ParserError::new(format!("Could not read MapProperty with types: {} {}", key_type, value_type)));
        }
        let num = reader.read_i32::<LittleEndian>()?;
        let mut map_data: Vec<(FPropertyTagType, FPropertyTagType)> = Vec::new();
        let err_f = |v| ParserError::add(v, format!("MapProperty error, types: {} {}", key_type, value_type));
        for _i in 0..num {
            map_data.push((
                read_map_value(reader, key_type, "StructProperty", name_map, import_map).map_err(err_f)?,
                read_map_value(reader, value_type, "StructProperty", name_map, import_map).map_err(err_f)?,
            ));
        }
        Ok(Self {
            map_data,
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

#[derive(Debug)]
enum FPropertyTagData {
    StructProperty (String, FGuid),
    BoolProperty (bool),
    ByteProperty (String),
    EnumProperty (String),
    ArrayProperty (String),
    MapProperty (String, String),
    SetProperty (String),
    NoData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FPropertyTagType {
    BoolProperty(bool),
    StructProperty(UScriptStruct),
    ObjectProperty(FPackageIndex),
    InterfaceProperty(UInterfaceProperty),
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
}

impl FPropertyTagType {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, 
                    property_type: &str, tag_data: Option<&FPropertyTagData>) -> ParserResult<Self> {
        Ok(match property_type {
            "BoolProperty" => FPropertyTagType::BoolProperty(
                match tag_data.unwrap() {
                    FPropertyTagData::BoolProperty(val) => val.clone(),
                    _ => panic!("Bool property does not have bool data"),
                }
            ),
            "StructProperty" => FPropertyTagType::StructProperty(
                match tag_data.unwrap() {
                    FPropertyTagData::StructProperty(name, _guid) => UScriptStruct::new(reader, name_map, import_map, name)?,
                    _ => panic!("Struct does not have struct data"),
                }
            ),
            "ObjectProperty" => FPropertyTagType::ObjectProperty(FPackageIndex::new_n(reader, name_map, import_map)?),
            "InterfaceProperty" => FPropertyTagType::InterfaceProperty(UInterfaceProperty::new_n(reader, name_map, import_map)?),
            "FloatProperty" =>  FPropertyTagType::FloatProperty(reader.read_f32::<LittleEndian>()?),
            "TextProperty" => FPropertyTagType::TextProperty(FText::new(reader)?),
            "StrProperty" => FPropertyTagType::StrProperty(read_string(reader)?),
            "NameProperty" => FPropertyTagType::NameProperty(read_fname(reader, name_map)?),
            "IntProperty" => FPropertyTagType::IntProperty(reader.read_i32::<LittleEndian>()?),
            "UInt16Property" => FPropertyTagType::UInt16Property(reader.read_u16::<LittleEndian>()?),
            "UInt32Property" => FPropertyTagType::UInt32Property(reader.read_u32::<LittleEndian>()?),
            "UInt64Property" => FPropertyTagType::UInt64Property(reader.read_u64::<LittleEndian>()?),
            "ArrayProperty" => match tag_data.unwrap() {
                FPropertyTagData::ArrayProperty(inner_type) => FPropertyTagType::ArrayProperty(
                    UScriptArray::new(reader, inner_type, name_map, import_map)?
                ),
                _ => panic!("Could not read array from non-array"),
            },
            "MapProperty" => match tag_data.unwrap() {
                FPropertyTagData::MapProperty(key_type, value_type) => FPropertyTagType::MapProperty(
                    UScriptMap::new(reader, name_map, import_map, key_type, value_type)?,
                ),
                _ => panic!("Map needs map data"),
            },
            "ByteProperty" => match tag_data.unwrap() {
                FPropertyTagData::ByteProperty(name) => {
                    if name == "None" { FPropertyTagType::ByteProperty(reader.read_u8()?) } else { FPropertyTagType::NameProperty(read_fname(reader, name_map)?) }
                },
                _ => panic!("Byte needs byte data"),
            },
            "EnumProperty" => FPropertyTagType::EnumProperty(
                match tag_data.unwrap() {
                    FPropertyTagData::EnumProperty(val) => {
                        if val == "None" { None } else { Some(read_fname(reader, name_map)?) }
                    },
                    _ => panic!("Enum property does not have enum data"),
                }
            ),
            "SoftObjectProperty" => FPropertyTagType::SoftObjectProperty(FSoftObjectPath::new_n(reader, name_map, import_map)?),
            _ => return Err(ParserError::new(format!("Could not read property type: {} at pos {}", property_type, reader.position()))),
        })
    }
}

#[derive(Debug)]
pub struct FPropertyTag {
    name: String,
    property_type: String,
    tag_data: FPropertyTagData,
    size: i32,
    array_index: i32,
    property_guid: Option<FGuid>,
    tag: Option<FPropertyTagType>,
}

impl FPropertyTag {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_data(&self) -> &FPropertyTagType {
        match &self.tag {
            Some(data) => data,
            None => panic!("no data"),
        }
    }
}

// I have no idea how to do this properly.
fn tag_data_overrides(tag_name: &str) -> Option<FPropertyTagData> {
    match tag_name {
        "BindingIdToReferences" => Some(FPropertyTagData::MapProperty("Guid".to_owned(), "LevelSequenceBindingReferenceArray".to_owned())),
        "Tracks" => Some(FPropertyTagData::MapProperty("MovieSceneTrackIdentifier".to_owned(), "MovieSceneEvaluationTrack".to_owned())),
        "SubTemplateSerialNumbers" => Some(FPropertyTagData::MapProperty("MovieSceneSequenceID".to_owned(), "UInt32Property".to_owned())),
        "SubSequences" => Some(FPropertyTagData::MapProperty("MovieSceneSequenceID".to_owned(), "MovieSceneSubSequenceData".to_owned())),
        "Hierarchy" => Some(FPropertyTagData::MapProperty("MovieSceneSequenceID".to_owned(), "MovieSceneSequenceHierarchyNode".to_owned())),
        "TrackSignatureToTrackIdentifier" => Some(FPropertyTagData::MapProperty("Guid".to_owned(), "MovieSceneTrackIdentifier".to_owned())),
        "SubSectionRanges" => Some(FPropertyTagData::MapProperty("Guid".to_owned(), "MovieSceneFrameRange".to_owned())),
        _ => None,
    }
}

fn read_property_tag(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, read_data: bool) -> ParserResult<Option<FPropertyTag>> {
    let name = read_fname(reader, name_map)?;
    if name == "None" {
        return Ok(None);
    }

    let property_type = read_fname(reader, name_map)?.trim().to_owned();
    let size = reader.read_i32::<LittleEndian>()?;
    let array_index = reader.read_i32::<LittleEndian>()?;

    let mut tag_data = match property_type.as_ref() {
        "StructProperty" => FPropertyTagData::StructProperty(read_fname(reader, name_map)?, FGuid::new(reader)?),
        "BoolProperty" => FPropertyTagData::BoolProperty(reader.read_u8()? != 0),
        "EnumProperty" => FPropertyTagData::EnumProperty(read_fname(reader, name_map)?),
        "ByteProperty" => FPropertyTagData::ByteProperty(read_fname(reader, name_map)?),
        "ArrayProperty" => FPropertyTagData::ArrayProperty(read_fname(reader, name_map)?),
        "MapProperty" => FPropertyTagData::MapProperty(read_fname(reader, name_map)?, read_fname(reader, name_map)?),
        "SetProperty" => FPropertyTagData::SetProperty(read_fname(reader, name_map)?),
        _ => FPropertyTagData::NoData,
    };

    // MapProperty doesn't seem to store the inner types as their types when they're UStructs.
    if property_type == "MapProperty" {
        tag_data = match tag_data_overrides(&name) {
            Some(data) => data,
            None => tag_data,
        };
    }

    let has_property_guid = reader.read_u8()? != 0;
    let property_guid = match has_property_guid {
        true => Some(FGuid::new(reader)?),
        false => None,
    };

    let property_desc = format!("Property Tag: {} ({})", name, property_type);

    let pos = reader.position();
    let tag = match read_data {
        true => Some(FPropertyTagType::new(reader, name_map, import_map, property_type.as_ref(), Some(&tag_data)).map_err(|v| ParserError::add(v, property_desc))?),
        false => None,
    };
    let final_pos = pos + (size as u64);
    if read_data {
        reader.seek(SeekFrom::Start(final_pos as u64)).expect("Could not seek to size");
    }
    if read_data && final_pos != reader.position() {
        println!("Could not read entire property: {} ({})", name, property_type);
    }

    Ok(Some(FPropertyTag {
        name,
        property_type,
        tag_data,
        size,
        array_index,
        property_guid,
        tag,
    }))
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

#[derive(Debug)]
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

impl FByteBulkData {
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>, bulk_offset: i64) -> ParserResult<Self> {
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
            // Archive seems "kind of" appended.
            let offset = header.offset_in_file + bulk_offset;
            data.resize(header.element_count as usize, 0u8);
            ubulk_reader.seek(SeekFrom::Start(offset as u64)).unwrap();
            ubulk_reader.read_exact(&mut data).unwrap();
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
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>, bulk_offset: i64) -> ParserResult<Self> {
        let cooked = reader.read_i32::<LittleEndian>()?;
        let data = FByteBulkData::new(reader, ubulk, bulk_offset)?;
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
}

impl FTexturePlatformData {
    
}

impl FTexturePlatformData {
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>, bulk_offset: i64) -> ParserResult<Self> {
        let size_x = reader.read_i32::<LittleEndian>()?;
        let size_y = reader.read_i32::<LittleEndian>()?;
        let num_slices = reader.read_i32::<LittleEndian>()?;
        let pixel_format = read_string(reader)?;
        let first_mip = reader.read_i32::<LittleEndian>()?;
        let length = reader.read_u32::<LittleEndian>()?;
        let mut mips = Vec::new();
        for _i in 0..length {
            mips.push(FTexture2DMipMap::new(reader, ubulk, bulk_offset)?);
        }

        Ok(Self {
            size_x, size_y, num_slices, pixel_format, first_mip, mips,
        })
    }
}

pub trait PackageExport: std::fmt::Debug + TraitSerialize {
    fn get_export_type(&self) -> &str;
}

serialize_trait_object!(PackageExport);

/// A UObject is a struct for all of the parsed properties of an object
#[derive(Debug)]
pub struct UObject {
    export_type: String,
    properties: Vec<FPropertyTag>,
}

impl UObject {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, export_type: &str) -> ParserResult<Self> {
        let properties = Self::serialize_properties(reader, name_map, import_map).map_err(|v| ParserError::add(v, format!("Export type: {}", export_type)))?;
        let serialize_guid = reader.read_u32::<LittleEndian>()? != 0;
        if serialize_guid {
            let _object_guid = FGuid::new(reader);
        }

        Ok(Self {
            properties: properties,
            export_type: export_type.to_owned(),
        })
    }

    fn serialize_properties(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Vec<FPropertyTag>> {
        let mut properties = Vec::new();
        loop {
            let tag = read_property_tag(reader, name_map, import_map, true)?;
            let tag = match tag {
                Some(data) => data,
                None => break,
            };

            properties.push(tag);
        }

        Ok(properties)
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
}

impl Serialize for UObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.properties.len() + 1))?;
        map.serialize_entry("export_type", &self.export_type)?;
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
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, asset_file_size: i32, export_size: i64, ubulk: &mut Option<ReaderCursor>) -> ParserResult<Self> {
        let object = UObject::new(reader, name_map, import_map, "Texture2D")?;

        FStripDataFlags::new(reader)?; // still no idea
        FStripDataFlags::new(reader)?; // why there are two

        let mut textures: Vec<FTexturePlatformData> = Vec::new();
        let cooked = reader.read_u32::<LittleEndian>()?;
        if cooked == 1 {
            let mut pixel_format = read_fname(reader, name_map)?;
            while pixel_format != "None" {
                let skip_offset = reader.read_i64::<LittleEndian>()?;
                let texture = FTexturePlatformData::new(reader, ubulk, export_size + asset_file_size as i64)?;
                if reader.position() + asset_file_size as u64 != skip_offset as u64 {
                    panic!("Texture read incorrectly {} {}", reader.position() + asset_file_size as u64, skip_offset);
                }
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
}

impl UDataTable {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "RowStruct")?;
        let num_rows = reader.read_i32::<LittleEndian>()?;

        let mut rows = Vec::new();

        for _i in 0..num_rows {
            let row_name = read_fname(reader, name_map)?;
            let row_object = UObject {
                properties: UObject::serialize_properties(reader, name_map, import_map)?,
                export_type: "RowStruct".to_owned(),
            };
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

#[derive(Debug, Serialize)]
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
                ECurveTableMode::RichCurves => "RichCurveKey,"
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
}

/// A Package is the collection of parsed data from a uasset/uexp file combo
/// 
/// It contains a number of 'Exports' which could be of any type implementing the `PackageExport` trait
/// Note that exports are of type `dyn Any` and will need to be downcasted to their appropriate types before being usable
pub struct Package {
    summary: FPackageFileSummary,
    exports: Vec<Box<Any>>,
}

#[allow(dead_code)]
impl Package {
    pub fn from_buffer(uasset: Vec<u8>, uexp: Vec<u8>, ubulk: Option<Vec<u8>>) -> ParserResult<Self> {
        let mut cursor = ReaderCursor::new(uasset);
        let summary = FPackageFileSummary::new(&mut cursor)?;

        let mut name_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.name_offset as u64))?;
        for _i in 0..summary.name_count {
            name_map.push(FNameEntrySerialized::new(&mut cursor)?);
        }

        let mut import_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.import_offset as u64))?;
        for _i in 0..summary.import_count {
            import_map.push(FObjectImport::new_n(&mut cursor, &name_map, &import_map)?);
        }

        let mut export_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.export_offset as u64))?;
        for _i in 0..summary.export_count {
            export_map.push(FObjectExport::new_n(&mut cursor, &name_map, &import_map)?);
        }

        let export_size = export_map.iter().fold(0, |acc, v| v.serial_size + acc);

        // read uexp file
        let mut cursor = ReaderCursor::new(uexp);

        let mut ubulk_cursor = match ubulk {
            Some(data) => Some(ReaderCursor::new(data)),
            None => None,
        };

        let asset_length = summary.total_header_size;

        let mut exports: Vec<Box<dyn Any>> = Vec::new();

        for v in &export_map {
            let export_type = &v.class_index.import;
            let position = v.serial_offset as u64 - asset_length as u64;
            cursor.seek(SeekFrom::Start(position))?;
            let export: Box<dyn Any> = match export_type.as_ref() {
                "Texture2D" => Box::new(Texture2D::new(&mut cursor, &name_map, &import_map, asset_length, export_size, &mut ubulk_cursor)?),
                "DataTable" => Box::new(UDataTable::new(&mut cursor, &name_map, &import_map)?),
                "SkeletalMesh" => Box::new(USkeletalMesh::new(&mut cursor, &name_map, &import_map)?),
                "AnimSequence" => Box::new(UAnimSequence::new(&mut cursor, &name_map, &import_map)?),
                "Skeleton" => Box::new(USkeleton::new(&mut cursor, &name_map, &import_map)?),
                "CurveTable" => Box::new(UCurveTable::new(&mut cursor, &name_map, &import_map)?),
                //"MaterialInstanceConstant" => Box::new(material_instance::UMaterialInstanceConstant::new(&mut cursor, &name_map, &import_map)?),
                _ => Box::new(UObject::new(&mut cursor, &name_map, &import_map, export_type)?),
            };
            let valid_pos = position + v.serial_size as u64;
            if cursor.position() != valid_pos {
                println!("Did not read {} correctly. Current Position: {}, Bytes Remaining: {}", export_type, cursor.position(), valid_pos as i64 - cursor.position() as i64);
            }
            exports.push(export);
        }

        Ok(Self {
            summary: summary,
            exports: exports,
        })
    }

    pub fn from_file(file_path: &str) -> ParserResult<Self> {
        let asset_file = file_path.to_owned() + ".uasset";
        let uexp_file = file_path.to_owned() + ".uexp";
        let ubulk_file = file_path.to_owned() + ".ubulk";

        // read asset file
        let mut asset = File::open(asset_file).map_err(|_v| ParserError::new(format!("Could not find file: {}", file_path)))?;
        let mut uasset_buf = Vec::new();
        asset.read_to_end(&mut uasset_buf)?;

        // read uexp file
        let mut uexp = File::open(uexp_file)?;
        let mut uexp_buf = Vec::new();
        uexp.read_to_end(&mut uexp_buf)?;

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

        Self::from_buffer(uasset_buf, uexp_buf, ubulk_buf)
    }

    pub fn get_exports(self) -> Vec<Box<Any>> {
        self.exports
    }

    /// Returns a reference to an export
    /// 
    /// Export will live as long as the underlying Package
    pub fn get_export(&self, index: usize) -> ParserResult<&dyn Any> {
        Ok(match self.exports.get(index) {
            Some(data) => data,
            None => return Err(ParserError::new(format!("index {} out of range", index))),
        }.as_ref())
    }

    pub fn get_export_move(mut self, index: usize) -> ParserResult<Box<dyn Any>> {
        if index < self.exports.len() {
            Ok(self.exports.swap_remove(index))
        } else {
            Err(ParserError::new(format!("No exports found")))
        }
    }

    pub fn empty() -> Self {
        Self {
            summary: FPackageFileSummary::empty(),
            exports: Vec::new(),
        }
    }
}

impl fmt::Debug for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for export in &self.exports {
            if let Some(obj) = get_export(export) {
                write!(f, "{:#?}", obj)?
            }
        }
        write!(f, "{:#?}", self.summary)
    }
}

// Still working out how I would ever do this properly. An enum doesn't seem quite right.
fn get_export(export: &Box<dyn Any>) -> Option<&PackageExport> {
    if let Some(obj) = export.downcast_ref::<UObject>() {
        return Some(obj);
    }
    if let Some(texture) = export.downcast_ref::<Texture2D>() {
        return Some(texture);
    }
    if let Some(table) = export.downcast_ref::<UDataTable>() {
        return Some(table);
    }
    if let Some(mesh) = export.downcast_ref::<USkeletalMesh>() {
        return Some(mesh);
    }
    if let Some(animation) = export.downcast_ref::<UAnimSequence>() {
        return Some(animation);
    }
    if let Some(skeleton) = export.downcast_ref::<USkeleton>() {
        return Some(skeleton);
    }
    if let Some(curve_table) = export.downcast_ref::<UCurveTable>() {
        return Some(curve_table);
    }
    if let Some(material) = export.downcast_ref::<material_instance::UMaterialInstanceConstant>() {
        return Some(material);
    }
    None
}

impl Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.summary.export_count as usize))?;
        for e in &self.exports {
            if let Some(obj) = get_export(e) {
                seq.serialize_element(obj)?;
                continue;
            }
            seq.serialize_element("None")?;
        }
        seq.end()
    }
}