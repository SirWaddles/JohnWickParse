use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::fs::File;
use std::any::Any;
use serde::ser::{Serialize, Serializer, SerializeMap, SerializeSeq};
use erased_serde::{Serialize as TraitSerialize};
use byteorder::{LittleEndian, ReadBytesExt};

pub type ReaderCursor = Cursor<Vec<u8>>;

pub trait Newable {
    fn new(reader: &mut ReaderCursor) -> Self;
}

#[derive(Debug, Serialize)]
pub struct FGuid {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

impl Newable for FGuid {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            a: reader.read_u32::<LittleEndian>().unwrap(),
            b: reader.read_u32::<LittleEndian>().unwrap(),
            c: reader.read_u32::<LittleEndian>().unwrap(),
            d: reader.read_u32::<LittleEndian>().unwrap(),
        }
    }
}

impl NewableWithNameMap for FGuid {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> Self {
        FGuid::new(reader)
    }
}

impl fmt::Display for FGuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:08x}{:08x}{:08x}{:08x}", self.a, self.b, self.c, self.d)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FCustomVersion {
    key: FGuid,
    version: i32,
}

impl Newable for FCustomVersion {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            key: FGuid::new(reader),
            version: reader.read_i32::<LittleEndian>().unwrap(),
        }
    }
}

pub fn read_string(reader: &mut ReaderCursor) -> String {
    let length = reader.read_i32::<LittleEndian>().unwrap();
    let mut bytes = vec![0u8; length as usize];
    reader.read_exact(&mut bytes).expect("Could not read string");
    bytes.pop();

    std::str::from_utf8(&bytes).unwrap().to_owned()
}

#[allow(dead_code)]
#[derive(Debug)]
struct FGenerationInfo {
    export_count: i32,
    name_count: i32,
}

impl Newable for FGenerationInfo {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            export_count: reader.read_i32::<LittleEndian>().unwrap(),
            name_count: reader.read_i32::<LittleEndian>().unwrap(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FEngineVersion {
    major: u16,
    minor: u16,
    patch: u16,
    changelist: u32,
    branch: String,
}

impl Newable for FEngineVersion {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            major: reader.read_u16::<LittleEndian>().unwrap(),
            minor: reader.read_u16::<LittleEndian>().unwrap(),
            patch: reader.read_u16::<LittleEndian>().unwrap(),
            changelist: reader.read_u32::<LittleEndian>().unwrap(),
            branch: read_string(reader),
        }
    }
}

pub fn read_tarray<S>(reader: &mut ReaderCursor) -> Vec<S> where S: Newable {
    let length = reader.read_u32::<LittleEndian>().unwrap();
    let mut container = Vec::new();

    for _i in 0..length {
        container.push(S::new(reader));
    }

    container
}

impl Newable for String {
    fn new(reader: &mut ReaderCursor) -> Self {
        read_string(reader)
    }
}

impl Newable for i32 {
    fn new(reader: &mut ReaderCursor) -> Self {
        reader.read_i32::<LittleEndian>().unwrap()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FCompressedChunk {
    uncompressed_offset: i32,
    uncompressed_size: i32,
    compressed_offset: i32,
    compressed_size: i32,
}

impl Newable for FCompressedChunk {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            uncompressed_offset: reader.read_i32::<LittleEndian>().unwrap(),
            uncompressed_size: reader.read_i32::<LittleEndian>().unwrap(),
            compressed_offset: reader.read_i32::<LittleEndian>().unwrap(),
            compressed_size: reader.read_i32::<LittleEndian>().unwrap(),
        }
    }
}

#[allow(dead_code)]
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

impl Newable for FPackageFileSummary {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            tag: reader.read_i32::<LittleEndian>().unwrap(),
            legacy_file_version: reader.read_i32::<LittleEndian>().unwrap(),
            legacy_ue3_version: reader.read_i32::<LittleEndian>().unwrap(),
            file_version_u34: reader.read_i32::<LittleEndian>().unwrap(),
            file_version_licensee_ue4: reader.read_i32::<LittleEndian>().unwrap(),
            custom_version_container: read_tarray(reader),
            total_header_size: reader.read_i32::<LittleEndian>().unwrap(),
            folder_name: read_string(reader),
            package_flags: reader.read_u32::<LittleEndian>().unwrap(),
            name_count: reader.read_i32::<LittleEndian>().unwrap(),
            name_offset: reader.read_i32::<LittleEndian>().unwrap(),
            gatherable_text_data_count: reader.read_i32::<LittleEndian>().unwrap(),
            gatherable_text_data_offset: reader.read_i32::<LittleEndian>().unwrap(),
            export_count: reader.read_i32::<LittleEndian>().unwrap(),
            export_offset: reader.read_i32::<LittleEndian>().unwrap(),
            import_count: reader.read_i32::<LittleEndian>().unwrap(),
            import_offset: reader.read_i32::<LittleEndian>().unwrap(),
            depends_offset: reader.read_i32::<LittleEndian>().unwrap(),
            string_asset_references_count: reader.read_i32::<LittleEndian>().unwrap(),
            string_asset_references_offset: reader.read_i32::<LittleEndian>().unwrap(),
            searchable_names_offset: reader.read_i32::<LittleEndian>().unwrap(),
            thumbnail_table_offset: reader.read_i32::<LittleEndian>().unwrap(),
            guid: FGuid::new(reader),
            generations: read_tarray(reader),
            saved_by_engine_version: FEngineVersion::new(reader),
            compatible_with_engine_version: FEngineVersion::new(reader),
            compression_flags: reader.read_u32::<LittleEndian>().unwrap(),
            compressed_chunks: read_tarray(reader),
            package_source: reader.read_u32::<LittleEndian>().unwrap(),
            additional_packages_to_cook: read_tarray(reader),
            asset_registry_data_offset: reader.read_i32::<LittleEndian>().unwrap(),
            buld_data_start_offset: reader.read_i32::<LittleEndian>().unwrap(),
            world_tile_info_data_offset: reader.read_i32::<LittleEndian>().unwrap(),
            chunk_ids: read_tarray(reader),
            preload_dependency_count: reader.read_i32::<LittleEndian>().unwrap(),
            preload_dependency_offset: reader.read_i32::<LittleEndian>().unwrap(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FNameEntrySerialized {
    data: String,
    non_case_preserving_hash: u16,
    case_preserving_hash: u16,
}

impl Newable for FNameEntrySerialized {
    fn new(reader: &mut ReaderCursor) -> Self {
        let mut length = reader.read_i32::<LittleEndian>().unwrap();
        let mut fstr;

        if length < 0 {
            length *= -1;
            let mut u16bytes = vec![0u16; length as usize];
            for i in 0..length {
                let val = reader.read_u16::<LittleEndian>().unwrap();
                u16bytes[i as usize] = val;
            }
            fstr = String::from_utf16(&u16bytes).expect("String parse failed");
        } else {
            let mut bytes = vec![0u8; length as usize];
            reader.read_exact(&mut bytes).expect("Could not read string");
            fstr = std::str::from_utf8(&bytes).unwrap().to_owned();
            fstr.pop();
        }

        Self {
            data: fstr,
            non_case_preserving_hash: reader.read_u16::<LittleEndian>().unwrap(),
            case_preserving_hash: reader.read_u16::<LittleEndian>().unwrap(),
        }
    }
}

type NameMap = Vec<FNameEntrySerialized>;
type ImportMap = Vec<FObjectImport>;

trait NewableWithNameMap: std::fmt::Debug + TraitSerialize {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> Self
    where Self: Sized;
}

serialize_trait_object!(NewableWithNameMap);

fn read_fname(reader: &mut ReaderCursor, name_map: &NameMap) -> String {
    let name_index = reader.read_i32::<LittleEndian>().unwrap();
    reader.read_i32::<LittleEndian>().unwrap(); // name_number ?
    name_map[name_index as usize].data.to_owned()
}

#[derive(Debug)]
struct FPackageIndex {
    index: i32,
    import: String,
}

#[allow(dead_code)]
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
}

impl NewableWithNameMap for FPackageIndex {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, import_map: &ImportMap) -> Self {
        let index = reader.read_i32::<LittleEndian>().unwrap();
        let import = match FPackageIndex::get_package(index, import_map) {
            Some(data) => data.object_name.clone(),
            None => "None".to_owned(),
        };
        Self {
            index,
            import,
        }
    }
}

impl Serialize for FPackageIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.import)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FObjectImport {
    class_package: String,
    class_name: String,
    outer_index: FPackageIndex,
    object_name: String,
}

impl NewableWithNameMap for FObjectImport {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> Self {
        Self {
            class_package: read_fname(reader, name_map),
            class_name: read_fname(reader, name_map),
            outer_index: FPackageIndex::new_n(reader, name_map, import_map),
            object_name: read_fname(reader, name_map),
        }
    }
}

impl Serialize for FObjectImport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.object_name)
    }
}

#[allow(dead_code)]
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
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> Self {
        Self {
            class_index: FPackageIndex::new_n(reader, name_map, import_map),
            super_index: FPackageIndex::new_n(reader, name_map, import_map),
            template_index: FPackageIndex::new_n(reader, name_map, import_map),
            outer_index: FPackageIndex::new_n(reader, name_map, import_map),
            object_name: read_fname(reader, name_map),
            save: reader.read_u32::<LittleEndian>().unwrap(),
            serial_size: reader.read_i64::<LittleEndian>().unwrap(),
            serial_offset: reader.read_i64::<LittleEndian>().unwrap(),
            forced_export: reader.read_i32::<LittleEndian>().unwrap() != 0,
            not_for_client: reader.read_i32::<LittleEndian>().unwrap() != 0,
            not_for_server: reader.read_i32::<LittleEndian>().unwrap() != 0,
            package_guid: FGuid::new(reader),
            package_flags: reader.read_u32::<LittleEndian>().unwrap(),
            not_always_loaded_for_editor_game: reader.read_i32::<LittleEndian>().unwrap() != 0,
            is_asset: reader.read_i32::<LittleEndian>().unwrap() != 0,
            first_export_dependency: reader.read_i32::<LittleEndian>().unwrap(),
            serialization_before_serialization_dependencies: reader.read_i32::<LittleEndian>().unwrap() != 0,
            create_before_serialization_dependencies: reader.read_i32::<LittleEndian>().unwrap() != 0,
            serialization_before_create_dependencies: reader.read_i32::<LittleEndian>().unwrap() != 0,
            create_before_create_dependencies: reader.read_i32::<LittleEndian>().unwrap() != 0,
        }
    }
}

impl Serialize for FObjectExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.object_name)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FText {
    flags: u32,
    history_type: i8,
    namespace: String,
    key: String,
    source_string: String,
}

impl Newable for FText {
    fn new(reader: &mut ReaderCursor) -> Self {
        let flags = reader.read_u32::<LittleEndian>().unwrap();
        let history_type = reader.read_i8().unwrap();
        if history_type != 0 {
            panic!("Could not read history type (FText): {}", history_type);
        }

        Self {
            flags,
            history_type,
            namespace: read_string(reader),
            key: read_string(reader),
            source_string: read_string(reader),
        }
    }
}

impl Serialize for FText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.source_string)
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct FSoftObjectPath {
    asset_path_name: String,
    sub_path_string: String,
}

impl NewableWithNameMap for FSoftObjectPath {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> Self {
        Self {
            asset_path_name: read_fname(reader, name_map),
            sub_path_string: read_string(reader),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct FGameplayTagContainer {
    gameplay_tags: Vec<String>,
}

impl NewableWithNameMap for FGameplayTagContainer {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> Self {
        let length = reader.read_u32::<LittleEndian>().unwrap();
        let mut container = Vec::new();

        for _i in 0..length {
            container.push(read_fname(reader, name_map));
        }

        Self {
            gameplay_tags: container,
        }
    }
}

#[derive(Debug, Serialize)]
struct FIntPoint {
    x: u32,
    y: u32,
}

impl NewableWithNameMap for FIntPoint {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> Self {
        Self {
            x: reader.read_u32::<LittleEndian>().unwrap(),
            y: reader.read_u32::<LittleEndian>().unwrap(),
        }
    }
}

#[derive(Debug, Serialize)]
struct FVector2D {
    x: f32,
    y: f32,
}

impl NewableWithNameMap for FVector2D {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> Self {
        Self {
            x: reader.read_f32::<LittleEndian>().unwrap(),
            y: reader.read_f32::<LittleEndian>().unwrap(),
        }
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
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> Self {
        Self {
            r: reader.read_f32::<LittleEndian>().unwrap(),
            g: reader.read_f32::<LittleEndian>().unwrap(),
            b: reader.read_f32::<LittleEndian>().unwrap(),
            a: reader.read_f32::<LittleEndian>().unwrap(),
        }
    }
}

#[derive(Debug)]
struct FStructFallback {
    properties: Vec<FPropertyTag>,
}

impl NewableWithNameMap for FStructFallback {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> Self {
        let mut properties = Vec::new();
        loop {
            let tag = read_property_tag(reader, name_map, import_map);
            let tag = match tag {
                Some(data) => data,
                None => break,
            };

            properties.push(tag);
        }
        
        Self {
            properties: properties,
        }
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

#[allow(dead_code)]
#[derive(Debug)]
struct UScriptStruct {
    struct_name: String,
    struct_type: Box<NewableWithNameMap>,
}

#[allow(dead_code)]
impl UScriptStruct {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, struct_name: &str) -> Self {
        let struct_type: Box<NewableWithNameMap> = match struct_name {
            "Vector2D" => Box::new(FVector2D::new_n(reader, name_map, import_map)),
            "LinearColor" => Box::new(FLinearColor::new_n(reader, name_map, import_map)),
            "GameplayTagContainer" => Box::new(FGameplayTagContainer::new_n(reader, name_map, import_map)),
            "IntPoint" => Box::new(FIntPoint::new_n(reader, name_map, import_map)),
            "Guid" => Box::new(FGuid::new(reader)),
            _ => Box::new(FStructFallback::new_n(reader, name_map, import_map)),
        };
        Self {
            struct_name: struct_name.to_owned(),
            struct_type: struct_type,
        }
    }
}

impl Serialize for UScriptStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.struct_type.serialize(serializer)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum FPropertyTagData {
    StructProperty (String, FGuid),
    BoolProperty (bool),
    ByteProperty (String),
    EnumProperty (String),
    ArrayProperty (String),
    NoData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum FPropertyTagType {
    BoolProperty(bool),
    StructProperty(UScriptStruct),
    ObjectProperty(FPackageIndex),
    FloatProperty(f32),
    TextProperty(FText),
    NameProperty(String),
    IntProperty(i32),
    ArrayProperty,
    ByteProperty(u8),
    EnumProperty(Option<String>),
    SoftObjectProperty(FSoftObjectPath),
}

#[allow(dead_code)]
impl FPropertyTagType {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, 
                    property_type: &str, tag_data: &FPropertyTagData) -> Self {
        match property_type {
            "BoolProperty" => FPropertyTagType::BoolProperty(
                match tag_data {
                    FPropertyTagData::BoolProperty(val) => val.clone(),
                    _ => panic!("Bool property does not have bool data"),
                }
            ),
            "StructProperty" => FPropertyTagType::StructProperty(
                match tag_data {
                    FPropertyTagData::StructProperty(name, _guid) => UScriptStruct::new(reader, name_map, import_map, name),
                    _ => panic!("Struct does not have struct data"),
                }
            ),
            "ObjectProperty" => FPropertyTagType::ObjectProperty(FPackageIndex::new_n(reader, name_map, import_map)),
            "FloatProperty" =>  FPropertyTagType::FloatProperty(reader.read_f32::<LittleEndian>().unwrap()),
            "TextProperty" => FPropertyTagType::TextProperty(FText::new(reader)),
            "NameProperty" => FPropertyTagType::NameProperty(read_fname(reader, name_map)),
            "IntProperty" => FPropertyTagType::IntProperty(reader.read_i32::<LittleEndian>().unwrap()),
            "ArrayProperty" => FPropertyTagType::ArrayProperty,
            "ByteProperty" => FPropertyTagType::ByteProperty(reader.read_u8().unwrap()),
            "EnumProperty" => FPropertyTagType::EnumProperty(
                match tag_data {
                    FPropertyTagData::EnumProperty(val) => {
                        if val == "None" { None } else { Some(read_fname(reader, name_map)) }
                    },
                    _ => panic!("Enum property does not have enum data"),
                }
            ),
            "SoftObjectProperty" => FPropertyTagType::SoftObjectProperty(FSoftObjectPath::new_n(reader, name_map, import_map)),
            _ => panic!("Could not read property type: {}", property_type),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FPropertyTag {
    name: String,
    property_type: String,
    tag_data: FPropertyTagData,
    size: i32,
    array_index: i32,
    property_guid: Option<FGuid>,
    tag: FPropertyTagType,
}

#[allow(dead_code)]
fn read_property_tag(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> Option<FPropertyTag> {
    let name = read_fname(reader, name_map);
    if name == "None" {
        return None;
    }

    let property_type = read_fname(reader, name_map).trim().to_owned();
    let size = reader.read_i32::<LittleEndian>().unwrap();
    let array_index = reader.read_i32::<LittleEndian>().unwrap();

    println!("Reading property: {} {}", property_type, name);

    let tag_data = match property_type.as_ref() {
        "StructProperty" => FPropertyTagData::StructProperty(read_fname(reader, name_map), FGuid::new(reader)),
        "BoolProperty" => FPropertyTagData::BoolProperty(reader.read_u8().unwrap() != 0),
        "EnumProperty" => FPropertyTagData::EnumProperty(read_fname(reader, name_map)),
        "ByteProperty" => FPropertyTagData::ByteProperty(read_fname(reader, name_map)),
        "ArrayProperty" => FPropertyTagData::ArrayProperty(read_fname(reader, name_map)),
        _ => FPropertyTagData::NoData,
    };

    let has_property_guid = reader.read_u8().unwrap() != 0;
    let property_guid = match has_property_guid {
        true => Some(FGuid::new(reader)),
        false => None,
    };

    let pos = reader.position();
    let tag = FPropertyTagType::new(reader, name_map, import_map, property_type.as_ref(), &tag_data);
    let final_pos = pos + (size as u64);
    reader.seek(SeekFrom::Start(final_pos as u64)).expect("Could not seek to size");

    Some(FPropertyTag {
        name,
        property_type,
        tag_data,
        size,
        array_index,
        property_guid,
        tag
    })
}

#[allow(dead_code)]
#[derive(Debug)]
struct FStripDataFlags {
    global_strip_flags: u8,
    class_strip_flags: u8,
}

impl Newable for FStripDataFlags {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            global_strip_flags: reader.read_u8().unwrap(),
            class_strip_flags: reader.read_u8().unwrap(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FByteBulkDataHeader {
    bulk_data_flags: i32,
    element_count: i32,
    size_on_disk: i32,
    offset_in_file: i64,
}

impl Newable for FByteBulkDataHeader {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            bulk_data_flags: reader.read_i32::<LittleEndian>().unwrap(),
            element_count: reader.read_i32::<LittleEndian>().unwrap(),
            size_on_disk: reader.read_i32::<LittleEndian>().unwrap(),
            offset_in_file: reader.read_i64::<LittleEndian>().unwrap(),
        }
    }
}

#[allow(dead_code)]
struct FByteBulkData {
    header: FByteBulkDataHeader,
    data: Vec<u8>
}

impl std::fmt::Debug for FByteBulkData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Header: {:?} {}", self.header, self.data.len())
    }
}

impl Newable for FByteBulkData {
    fn new(reader: &mut ReaderCursor) -> Self {
        let header = FByteBulkDataHeader::new(reader);
        let mut data: Vec<u8> = Vec::new();

        if header.bulk_data_flags & 0x0040 != 0 {
            data.resize(header.element_count as usize, 0u8);
            reader.read_exact(&mut data).unwrap();
        }

        Self {
            header, data
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FTexture2DMipMap {
    data: FByteBulkData,
    size_x: i32,
    size_y: i32,
    size_z: i32,
}

impl Newable for FTexture2DMipMap {
    fn new(reader: &mut ReaderCursor) -> Self {
        let cooked = reader.read_i32::<LittleEndian>().unwrap();
        let data = FByteBulkData::new(reader);
        let size_x = reader.read_i32::<LittleEndian>().unwrap();
        let size_y = reader.read_i32::<LittleEndian>().unwrap();
        let size_z = reader.read_i32::<LittleEndian>().unwrap();
        if cooked != 1 {
            read_string(reader);
        }

        Self {
            data, size_x, size_y, size_z
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FTexturePlatformData {
    size_x: i32,
    size_y: i32,
    num_slices: i32,
    pixel_format: String,
    first_mip: i32,
    mips: Vec<FTexture2DMipMap>,
}

impl Newable for FTexturePlatformData {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            size_x: reader.read_i32::<LittleEndian>().unwrap(),
            size_y: reader.read_i32::<LittleEndian>().unwrap(),
            num_slices: reader.read_i32::<LittleEndian>().unwrap(),
            pixel_format: read_string(reader),
            first_mip: reader.read_i32::<LittleEndian>().unwrap(),
            mips: read_tarray(reader),
        }
    }
}

pub trait PackageExport: std::fmt::Debug {
    fn get_export_type(&self) -> &str;
}

#[allow(dead_code)]
#[derive(Debug)]
struct UObject {
    export_type: String,
    properties: Vec<FPropertyTag>,
}

#[allow(dead_code)]
impl UObject {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, export_type: &str) -> Option<Self> {
        println!("Export type: {}", export_type);
        let mut properties = Vec::new();
        loop {
            let tag = read_property_tag(reader, name_map, import_map);
            let tag = match tag {
                Some(data) => data,
                None => break,
            };

            properties.push(tag);
        }

        Some(Self {
            properties: properties,
            export_type: export_type.to_owned(),
        })
    }
}

impl Serialize for UObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.properties.len()))?;
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct Texture2D {
    base_object: UObject,
    cooked: u32,
    textures: Vec<FTexturePlatformData>,
}

impl Texture2D {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, asset_file_size: usize) -> Self {
        let object = UObject::new(reader, name_map, import_map, "Texture2D").unwrap();
        let serialize_guid = reader.read_u32::<LittleEndian>().unwrap();
        if serialize_guid != 0 {
            let _object_guid = FGuid::new(reader);
        }

        FStripDataFlags::new(reader); // still no idea
        FStripDataFlags::new(reader); // why there are two

        let mut textures: Vec<FTexturePlatformData> = Vec::new();
        let cooked = reader.read_u32::<LittleEndian>().unwrap();
        if cooked == 1 {
            let mut pixel_format = read_fname(reader, name_map);
            while pixel_format != "None" {
                let skip_offset = reader.read_i64::<LittleEndian>().unwrap();
                let texture = FTexturePlatformData::new(reader);
                if reader.position() + asset_file_size as u64 != skip_offset as u64 {
                    panic!("Texture read incorrectly {} {}", reader.position() + asset_file_size as u64, skip_offset);
                }
                textures.push(texture);
                pixel_format = read_fname(reader, name_map);
            }
        }

        Self {
            base_object: object,
            cooked: cooked,
            textures: textures,
        }
    }
}

impl PackageExport for Texture2D {
    fn get_export_type(&self) -> &str {
        "Texture2D"
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Package {
    summary: FPackageFileSummary,
    name_map: NameMap,
    import_map: ImportMap,
    export_map: Vec<FObjectExport>,
    asset_file_size: usize,
    exports: Vec<Box<Any>>,
}

#[allow(dead_code)]
impl Package {
    pub fn new(file_path: &str) -> Self {
        let asset_file = file_path.to_owned() + ".uasset";
        let uexp_file = file_path.to_owned() + ".uexp";
        // read asset file
        let mut asset = File::open(asset_file).unwrap();
        let mut buffer = Vec::new();

        asset.read_to_end(&mut buffer).expect("Could not read file");
        let asset_length = buffer.len();
        let mut cursor = ReaderCursor::new(buffer);
        let summary = FPackageFileSummary::new(&mut cursor);

        let mut name_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.name_offset as u64)).unwrap();
        for _i in 0..summary.name_count {
            name_map.push(FNameEntrySerialized::new(&mut cursor));
        }

        let mut import_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.import_offset as u64)).unwrap();
        for _i in 0..summary.import_count {
            import_map.push(FObjectImport::new_n(&mut cursor, &name_map, &import_map));
        }

        let mut export_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.export_offset as u64)).unwrap();
        for _i in 0..summary.export_count {
            export_map.push(FObjectExport::new_n(&mut cursor, &name_map, &import_map));
        }

        // read uexp file
        let mut uexp = File::open(uexp_file).unwrap();
        let mut buffer = Vec::new();
        uexp.read_to_end(&mut buffer).expect("Could not read uexp file");
        let mut cursor = ReaderCursor::new(buffer);

        let exports: Vec<Box<dyn Any>> = (&export_map).into_iter().map(|v| {
            let export_type = &v.class_index.import;
            let position = v.serial_offset as u64 - asset_length as u64;
            cursor.seek(SeekFrom::Start(position)).unwrap();
            let export: Box<dyn Any> = match export_type.as_ref() {
                "Texture2D" => Box::new(Texture2D::new(&mut cursor, &name_map, &import_map, asset_length)),
                _ => Box::new(UObject::new(&mut cursor, &name_map, &import_map, export_type).unwrap()),
            };
            let _zero = cursor.read_u32::<LittleEndian>().unwrap();
            if cursor.position() != position + v.serial_size as u64 {
                println!("Did not read {} correctly", export_type);
            }
            export
        }).collect();

        Self {
            summary: summary,
            name_map: name_map,
            import_map: import_map,
            export_map: export_map,
            asset_file_size: asset_length,
            exports: exports,
        }
    }

    pub fn get_export(&self) -> &dyn Any {
        self.exports.get(0).unwrap().as_ref()
    }
}

impl Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.summary.export_count as usize))?;
        for e in &self.exports {
            if let Some(obj) = e.downcast_ref::<UObject>() {
                seq.serialize_element(obj)?;
            }
            if let Some(texture) = e.downcast_ref::<Texture2D>() {
                seq.serialize_element(&texture.base_object)?;
            }
        }
        seq.end()
    }
}