extern crate byteorder;

use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::fs::File;
use byteorder::{LittleEndian, ReadBytesExt};

type ReaderCursor = Cursor<Vec<u8>>;

trait Newable {
    fn new(reader: &mut ReaderCursor) -> Self;
}

struct FGuid {
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

impl fmt::Display for FGuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:08x}{:08x}{:08x}{:08x}", self.a, self.b, self.c, self.d)
    }
}

#[allow(dead_code)]
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

fn read_string(reader: &mut ReaderCursor) -> String {
    let length = reader.read_i32::<LittleEndian>().unwrap();
    let mut bytes = vec![0u8; length as usize];
    reader.read_exact(&mut bytes).expect("Could not read string");

    std::str::from_utf8(&bytes).unwrap().to_owned()
}

#[allow(dead_code)]
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

fn read_tarray<S>(reader: &mut ReaderCursor) -> Vec<S> where S: Newable {
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

trait NewableWithNameMap {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap) -> Self;
}

fn read_fname(reader: &mut ReaderCursor, name_map: &NameMap) -> String {
    let name_index = reader.read_i32::<LittleEndian>().unwrap();
    reader.read_i32::<LittleEndian>().unwrap(); // name_number ?
    name_map[name_index as usize].data.to_owned()
}

struct FPackageIndex {
    index: i32,
}

impl FPackageIndex {
    fn is_import(&self) -> bool {
        self.index < 0
    }

    fn is_export(&self) -> bool {
        self.index > 0
    }

    fn is_null(&self) -> bool {
        self.index == 0
    }

    fn to_import(&self) -> i32 {
        self.index * -1 - 1
    }

    fn to_export(&self) -> i32 {
        self.index - 1
    }

    fn get_package<'a>(&self, import_map: &'a ImportMap) -> &'a FObjectImport {
        if self.is_import() {
            return import_map.get(self.to_import() as usize).unwrap();
        } else {
            return import_map.get(self.to_export() as usize).unwrap();
        }
    }
}

impl Newable for FPackageIndex {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            index: reader.read_i32::<LittleEndian>().unwrap(),
        }
    }
}

#[allow(dead_code)]
struct FObjectImport {
    class_package: String,
    class_name: String,
    outer_index: FPackageIndex,
    object_name: String,
}

impl NewableWithNameMap for FObjectImport {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap) -> Self {
        Self {
            class_package: read_fname(reader, name_map),
            class_name: read_fname(reader, name_map),
            outer_index: FPackageIndex::new(reader),
            object_name: read_fname(reader, name_map),
        }
    }
}

type ImportMap = Vec<FObjectImport>;

#[allow(dead_code)]
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
    fn new(reader: &mut ReaderCursor, name_map: &NameMap) -> Self {
        Self {
            class_index: FPackageIndex::new(reader),
            super_index: FPackageIndex::new(reader),
            template_index: FPackageIndex::new(reader),
            outer_index: FPackageIndex::new(reader),
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

#[allow(dead_code)]
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

#[allow(dead_code)]
struct FSoftObjectPath {
    asset_path_name: String,
    sub_path_string: String,
}

impl NewableWithNameMap for FSoftObjectPath {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap) -> Self {
        Self {
            asset_path_name: read_fname(reader, name_map),
            sub_path_string: read_string(reader),
        }
    }
}

enum FPropertyTagData {
    StructProperty (String, FGuid),
    BoolProperty (bool),
    ByteProperty (String),
    EnumProperty (String),
    ArrayProperty (String),
    NoData,
}

enum FPropertyTagType {
    BoolProperty(bool),
    StructProperty,
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

impl FPropertyTagType {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, property_type: &str, tag_data: &FPropertyTagData) -> Self {
        match property_type {
            "BoolProperty" => FPropertyTagType::BoolProperty(
                match tag_data {
                    FPropertyTagData::BoolProperty(val) => val.clone(),
                    _ => panic!("Bool property does not have bool data"),
                }
            ),
            "StructProperty" => FPropertyTagType::StructProperty,
            "ObjectProperty" => FPropertyTagType::ObjectProperty(FPackageIndex::new(reader)),
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
            "SoftObjectProperty" => FPropertyTagType::SoftObjectProperty(FSoftObjectPath::new(reader, name_map)),
            _ => panic!("Could not read property type: {}", property_type),
        }
    }
}

#[allow(dead_code)]
struct FPropertyTag {
    name: String,
    property_type: String,
    tag_data: FPropertyTagData,
    size: i32,
    array_index: i32,
    property_guid: Option<FGuid>,
    tag: FPropertyTagType,
}

fn read_property_tag(reader: &mut ReaderCursor, name_map: &NameMap) -> Option<FPropertyTag> {
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
    let tag = FPropertyTagType::new(reader, name_map, property_type.as_ref(), &tag_data);
    let final_pos = pos + (size as u64);
    println!("Seeking from {} to {}", reader.position(), final_pos);
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
struct UObject {
    export_type: String,
    properties: Vec<FPropertyTag>,
}

impl UObject {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, export_type: &str) -> Option<Self> {
        println!("Export type: {}", export_type);
        let mut properties = Vec::new();
        loop {
            let tag = read_property_tag(reader, name_map);
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

#[allow(dead_code)]
struct Package {
    summary: FPackageFileSummary,
    name_map: NameMap,
    import_map: ImportMap,
    export_map: Vec<FObjectExport>,
    asset_file_size: usize,
    exports: Vec<UObject>,
}

impl Package {
    fn new(asset_file: &str, uexp_file: &str) -> Self {
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
            import_map.push(FObjectImport::new(&mut cursor, &name_map));
        }

        let mut export_map = Vec::new();
        cursor.seek(SeekFrom::Start(summary.export_offset as u64)).unwrap();
        for _i in 0..summary.export_count {
            export_map.push(FObjectExport::new(&mut cursor, &name_map));
        }

        // read uexp file
        let mut uexp = File::open(uexp_file).unwrap();
        let mut buffer = Vec::new();
        uexp.read_to_end(&mut buffer).expect("Could not read uexp file");
        let mut cursor = ReaderCursor::new(buffer);

        let export_type = &export_map.get(0).unwrap().class_index.get_package(&import_map).object_name;
        let export = UObject::new(&mut cursor, &name_map, export_type);
        let exports = vec![export.unwrap()];


        Self {
            summary: summary,
            name_map: name_map,
            import_map: import_map,
            export_map: export_map,
            asset_file_size: asset_length,
            exports: exports,
        }
    }
}


fn main() {
    let test_package = Package::new("bid_024_space.uasset", "bid_024_space.uexp");
}
