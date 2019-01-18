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

fn read_tarray_string(reader: &mut ReaderCursor) -> Vec<String> {
    let length = reader.read_u32::<LittleEndian>().unwrap();
    let mut container = Vec::new();

    for _i in 0..length {
        container.push(read_string(reader));
    }

    container
}

fn read_tarray_i32(reader: &mut ReaderCursor) -> Vec<i32> {
    let length = reader.read_u32::<LittleEndian>().unwrap();
    let mut container = Vec::new();

    for _i in 0..length {
        container.push(reader.read_i32::<LittleEndian>().unwrap());
    }

    container
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
            additional_packages_to_cook: read_tarray_string(reader),
            asset_registry_data_offset: reader.read_i32::<LittleEndian>().unwrap(),
            buld_data_start_offset: reader.read_i32::<LittleEndian>().unwrap(),
            world_tile_info_data_offset: reader.read_i32::<LittleEndian>().unwrap(),
            chunk_ids: read_tarray_i32(reader),
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
            let mut bytes = vec![0u8; (length * 2) as usize];
            reader.read_exact(&mut bytes).expect("Could not read wide string");
            let mut slice = &bytes[..];
            let mut u16bytes = Vec::new();
            for i in 0..length {
                let val = slice.read_u16::<LittleEndian>().unwrap();
                u16bytes[i as usize] = val;
            }
            fstr = String::from_utf16(&u16bytes).expect("String parse failed");
        } else {
            let mut bytes = vec![0u8; length as usize];
            reader.read_exact(&mut bytes).expect("Could not read string");
            fstr = std::str::from_utf8(&bytes).unwrap().to_owned();
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
}

impl Newable for FPackageIndex {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            index: reader.read_i32::<LittleEndian>().unwrap(),
        }
    }
}

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
struct Package {
    summary: FPackageFileSummary,
    name_map: NameMap,
    import_map: ImportMap,
}

impl Package {
    fn new(asset_file: &str) -> Self {
        let mut asset = File::open(asset_file).unwrap();
        let mut buffer = Vec::new();

        asset.read_to_end(&mut buffer).expect("Could not read file");
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


        Self {
            summary: summary,
            name_map: name_map,
            import_map: import_map,
        }
    }
}


fn main() {
    let test_package = Package::new("bid_024_space.uasset");

    for name in test_package.import_map {
        println!("Name: {}", name.class_name);
    }

}
