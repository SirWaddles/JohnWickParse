use byteorder::{LittleEndian, ReadBytesExt};
use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::sync::Arc;
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};
use aes_soft::Aes256;
use flate2::read::ZlibDecoder;
use crate::assets::{FMappedName, FGuid, FPackageObjectIndex, Newable, ReaderCursor, read_string, read_short_string, read_tarray, ParserResult, ParserError};
use crate::decompress::oodle;

const MAX_INT: u32 = 4294967295;

#[derive(Debug)]
struct FIoDirectoryIndexEntry {
    name: u32,
    first_child: u32,
    next_sibling: u32,
    first_file: u32,
}

impl Newable for FIoDirectoryIndexEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            name: reader.read_u32::<LittleEndian>()?,
            first_child: reader.read_u32::<LittleEndian>()?,
            next_sibling: reader.read_u32::<LittleEndian>()?,
            first_file: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FIoFileIndexEntry {
    name: u32,
    next_file_entry: u32,
    user_data: u32,
}

impl Newable for FIoFileIndexEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            name: reader.read_u32::<LittleEndian>()?,
            next_file_entry: reader.read_u32::<LittleEndian>()?,
            user_data: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct FIoDirectoryIndexResource {
    mount_point: String,
    directories: Vec<FIoDirectoryIndexEntry>,
    files: Vec<FIoFileIndexEntry>,
    string_table: Vec<String>,
}

impl FIoDirectoryIndexResource {
    fn empty() -> Self {
        Self {
            mount_point: "".to_owned(),
            directories: Vec::new(),
            files: Vec::new(),
            string_table: Vec::new(),
        }
    }

    pub fn get_files(&self, size: usize) -> Vec<String> {
        let mut files = vec!["".to_owned(); size];

        let dir = &self.directories[0];
        self.process_directory(dir, &mut files, "".to_owned());

        files
    }

    fn process_directory(&self, dir: &FIoDirectoryIndexEntry, filenames: &mut Vec<String>, mut dirpath: String) {
        if dir.name != MAX_INT {
            dirpath += &self.string_table[dir.name as usize];
            dirpath += "/";
        }

        if dir.first_child != MAX_INT {
            let children = self.get_directories(&self.directories[dir.first_child as usize]);
            for child in children {
                self.process_directory(&child, filenames, dirpath.clone());
            }
        }

        if dir.first_file != MAX_INT {
            let files = self.get_file_entries(&self.files[dir.first_file as usize]);
            for file in files {
                if file.name != MAX_INT && file.user_data != MAX_INT {
                    let filepath = dirpath.clone() + &self.string_table[file.name as usize];
                    filenames[file.user_data as usize] = filepath;
                }
            }
        }
    }

    fn get_directories<'a>(&'a self, dir: &'a FIoDirectoryIndexEntry) -> Vec<&'a FIoDirectoryIndexEntry> {
        let mut dirs = Vec::new();
        let mut active = dir;
        dirs.push(active);
        while active.next_sibling != MAX_INT {
            active = &self.directories[active.next_sibling as usize];
            dirs.push(active);
        }

        dirs
    }

    fn get_file_entries<'a>(&'a self, file: &'a FIoFileIndexEntry) -> Vec<&'a FIoFileIndexEntry> {
        let mut files = Vec::new();
        let mut active = file;
        files.push(active);
        while active.next_file_entry != MAX_INT {
            active = &self.files[active.next_file_entry as usize];
            files.push(active);
        }

        files
    }
}

impl Newable for FIoDirectoryIndexResource {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            mount_point: read_string(reader)?,
            directories: read_tarray(reader)?,
            files: read_tarray(reader)?,
            string_table: read_tarray(reader)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FIoStoreTocHeader {
    version: u32,
    header_size: u32,
    entry_count: u32,
    compressed_block_entry_count: u32,
    compressed_block_entry_size: u32,
    compression_method_name_count: u32,
    compression_method_name_length: u32,
    compression_block_size: u32,
    directory_index_size: u32,
    something: u32,
    container_id: u64,
    encryption_key_guid: FGuid,
    container_flags: u32,
}

impl FIoStoreTocHeader {
    fn is_encrypted(&self) -> bool {
        self.container_flags & (1 << 1) != 0
    }

    fn is_signed(&self) -> bool {
        self.container_flags & (1 << 2) != 0
    }

    pub fn get_block_size(&self) -> u32 {
        self.compression_block_size
    }

    pub fn get_key_guid(&self) -> &FGuid {
        &self.encryption_key_guid
    }
}

impl Newable for FIoStoreTocHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut magic_img = [0u8; 16];
        reader.read_exact(&mut magic_img)?;
        let result = Self {
            version: reader.read_u32::<LittleEndian>()?,
            header_size: reader.read_u32::<LittleEndian>()?,
            entry_count: reader.read_u32::<LittleEndian>()?,
            compressed_block_entry_count: reader.read_u32::<LittleEndian>()?,
            compressed_block_entry_size: reader.read_u32::<LittleEndian>()?,
            compression_method_name_count: reader.read_u32::<LittleEndian>()?,
            compression_method_name_length: reader.read_u32::<LittleEndian>()?,
            compression_block_size: reader.read_u32::<LittleEndian>()?,
            directory_index_size: reader.read_u32::<LittleEndian>()?,
            something: reader.read_u32::<LittleEndian>()?,
            container_id: reader.read_u64::<LittleEndian>()?,
            encryption_key_guid: FGuid::new(reader)?,
            container_flags: reader.read_u32::<LittleEndian>()?,
        };

        let mut padding = [0u8; 60];
        reader.read_exact(&mut padding)?;

        Ok(result)
    }
}

#[derive(Debug)]
enum EIoChunkType {
    Invalid,
    InstallManifest,
    ExportBundleData,
    BulkData,
    OptionalBulkData,
    MemoryMappedBulkData,
    LoaderGlobalMeta,
    LoaderInitialLoadMeta,
    LoaderGlobalNames,
    LoaderGlobalNameHashes,
    ContainerHeader,
}

#[derive(Debug)]
pub struct FIoChunkId {
    id: u64,
    index: u32,
    chunk_type: EIoChunkType,
}

impl FIoChunkId {
    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl Newable for FIoChunkId {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let id = reader.read_u64::<LittleEndian>()?;
        let index = reader.read_u16::<LittleEndian>()? as u32;
        let _zero = reader.read_u8()?;

        let chunk_type = match reader.read_u8()? {
            0 => EIoChunkType::Invalid,
            1 => EIoChunkType::InstallManifest,
            2 => EIoChunkType::ExportBundleData,
            3 => EIoChunkType::BulkData,
            4 => EIoChunkType::OptionalBulkData,
            5 => EIoChunkType::MemoryMappedBulkData,
            6 => EIoChunkType::LoaderGlobalMeta,
            7 => EIoChunkType::LoaderInitialLoadMeta,
            8 => EIoChunkType::LoaderGlobalNames,
            9 => EIoChunkType::LoaderGlobalNameHashes,
            10 => EIoChunkType::ContainerHeader,
            _ => panic!("Unknown Chunk ID"),
        };
        

        Ok(Self {
            id, index, chunk_type,
        })
    }
}

#[derive(Debug)]
pub struct FIoOffsetAndLength {
    pub offset: u64,
    pub length: u64,
}

impl Newable for FIoOffsetAndLength {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut data = [0u8; 10];
        reader.read_exact(&mut data)?;

        let offset = data[4] as u64
            | ((data[3] as u64) << 8)
            | ((data[2] as u64) << 16)
            | ((data[1] as u64) << 24)
            | ((data[0] as u64) << 32);

        let length = data[9] as u64
            | ((data[8] as u64) << 8)
            | ((data[7] as u64) << 16)
            | ((data[6] as u64) << 24)
            | ((data[5] as u64) << 32);

        Ok(Self {
            offset,
            length,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FIoStoreTocCompressedBlockEntry {
    pub offset: u64,
    pub size: u32,
    pub compressed_size: u32,
    pub compression_method: u8,
}

impl Newable for FIoStoreTocCompressedBlockEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut data = [0u8; 12];
        reader.read_exact(&mut data)?;

        let offset_bits: u64 = 40;
        let offset_mask: u64 = (1 << offset_bits) - 1;
        let offset = u64::from_le_bytes(data[0..8].try_into().unwrap()) & offset_mask;

        let size_bits: u32 = 24;
        let size_mask: u32 = (1 << size_bits) - 1;
        let size_shift: u32 = 8;

        let compressed_size = (u32::from_le_bytes(data[4..8].try_into().unwrap()) >> size_shift) & size_mask;
        let size = u32::from_le_bytes(data[8..12].try_into().unwrap()) & size_mask;
        let compression_method = data[11];

        Ok(Self {
            offset,
            size,
            compressed_size,
            compression_method,
        })
    }
}

#[derive(Debug)]
struct FSHAHash {
    content: [u8; 20],
}

impl Newable for FSHAHash {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut content = [0u8; 20];
        reader.read_exact(&mut content)?;

        Ok(Self {
            content,
        })
    }
}

pub fn align_value(x: u32, a: u32) -> u32 {
    let r = x % a;
    if r != 0 { x + (a - r) } else { x }
}

fn get_chunk(file: &mut File, chunk: &FIoStoreTocCompressedBlockEntry, header: &FIoStoreTocHeader, key: &Option<Vec<u8>>) -> ParserResult<Vec<u8>> {
    file.seek(SeekFrom::Start(chunk.offset))?;

    let chunk_size = align_value(chunk.compressed_size, 16);

    let mut buf = vec![0u8; chunk_size as usize];
    file.read_exact(&mut buf)?;

    if header.is_encrypted() {
        let hex_key = match key {
            Some(k) => k,
            None => return Err(ParserError::new(format!("Key not specified"))),
        };

        let decrypt = Ecb::<Aes256, ZeroPadding>::new_var(&hex_key, Default::default()).unwrap();
        decrypt.decrypt(&mut buf).unwrap();
    }

    if chunk.compression_method == 0 {
        return Ok(buf);
    }

    Ok(oodle::decompress_stream(chunk.size as u64, &buf).unwrap())
}

pub struct ReaderData {
    compressed_blocks: Vec<FIoStoreTocCompressedBlockEntry>,
    compression_methods: Vec<String>,
    header: FIoStoreTocHeader,
    key: Option<Vec<u8>>,
}

impl ReaderData {
    pub fn get_header(&self) -> &FIoStoreTocHeader {
        &self.header
    }

    pub fn get_block(&self, idx: usize) -> Option<&FIoStoreTocCompressedBlockEntry> {
        self.compressed_blocks.get(idx)
    }
}

struct UcasReader {
    data: Arc<ReaderData>,
    current_chunk: usize,
    current_chunk_data: Vec<u8>,
    current_offset: usize,
    current_total_offset: u64,
    total_size: u64,
    handle: File,
}

impl UcasReader {
    fn new(path: &str, data: Arc<ReaderData>) -> ParserResult<Self> {
        let ucas_path = path.to_owned() + ".ucas";
        let mut file = File::open(ucas_path)?;

        let total_size = data.compressed_blocks.iter().fold(0, |acc, v| acc + v.size as u64);

        let first_chunk = get_chunk(&mut file, &data.compressed_blocks[0], &data.header, &data.key)?;
        Ok(Self {
            data,
            current_chunk: 0,
            current_offset: 0,
            handle: file,
            current_chunk_data: first_chunk,
            current_total_offset: 0,
            total_size: total_size,
        })
    }
}

impl Read for UcasReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.current_offset >= self.current_chunk_data.len() {
            self.current_chunk += 1;
            if self.current_chunk >= self.data.compressed_blocks.len() {
                return Ok(0);
            }

            self.current_offset = 0;
            let new_chunk = &self.data.compressed_blocks[self.current_chunk];
            self.current_chunk_data = match get_chunk(&mut self.handle, &new_chunk, &self.data.header, &self.data.key) {
                Ok(d) => d,
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, e)),
            };
        }

        let target_len = std::cmp::min(buf.len(), self.current_chunk_data.len() - self.current_offset);

        let target_buf = &mut buf[0..target_len];
        target_buf.copy_from_slice(&self.current_chunk_data[self.current_offset..(self.current_offset + target_len)]);

        self.current_total_offset += target_len as u64;
        self.current_offset += target_len;

        Ok(target_len)
    }
}

impl Seek for UcasReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(p) => (self.current_total_offset as i64 + p) as u64,
            SeekFrom::End(p) => (self.total_size as i64 + p) as u64,
        };

        let block_size = self.data.header.compression_block_size as u64;

        self.current_total_offset = target;
        self.current_chunk = (target / block_size) as usize;
        self.current_offset = (target % block_size) as usize;
        self.current_chunk_data = get_chunk(&mut self.handle, &self.data.compressed_blocks[self.current_chunk], &self.data.header, &self.data.key).unwrap();

        Ok(self.current_total_offset)
    }
}

#[derive(Debug)]
struct FPackageStoreEntry {
    export_bundle_size: u64,
    export_count: i32,
    export_bundle_count: i32,
    load_order: u32,
    pad: u32,
    imported_packages: Vec<u64>,
}

impl Newable for FPackageStoreEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let export_bundle_size = reader.read_u64::<LittleEndian>()?;
        let export_count = reader.read_i32::<LittleEndian>()?;
        let export_bundle_count = reader.read_i32::<LittleEndian>()?;
        let load_order = reader.read_u32::<LittleEndian>()?;
        let pad = reader.read_u32::<LittleEndian>()?;

        let cpos = reader.position();
        let package_num = reader.read_u32::<LittleEndian>()?;
        let package_offset = reader.read_u32::<LittleEndian>()?;
        let mut imported_packages = Vec::new();

        if package_num != 0 {
            reader.seek(SeekFrom::Start(package_offset as u64 + cpos as u64))?;

            for _i in 0..package_num {
                imported_packages.push(reader.read_u64::<LittleEndian>()?);
            }

            reader.seek(SeekFrom::Start(cpos + 8))?;
        }

        Ok(Self {
            export_bundle_size,
            export_count,
            export_bundle_count,
            load_order,
            pad,
            imported_packages,
        })
    }
}

#[derive(Debug)]
pub struct FContainerHeader {
    container_id: u64,
    package_count: u32,
    names: Vec<u8>,
    name_hashes: Vec<u8>,
    package_ids: Vec<u64>,
    packages: Vec<FPackageStoreEntry>,
}

impl Newable for FContainerHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let container_id = reader.read_u64::<LittleEndian>()?;
        let package_count = reader.read_u32::<LittleEndian>()?;
        let names = read_tarray(reader)?;
        let name_hashes = read_tarray(reader)?;
        let package_ids = read_tarray(reader)?;

        let store_buf: Vec<u8> = read_tarray(reader)?;
        let mut store_reader = Cursor::new(store_buf.as_slice());
        let mut packages = Vec::new();
        for _i in 0..package_count {
            let package = FPackageStoreEntry::new(&mut store_reader)?;
            packages.push(package);
        }

        Ok(Self {
            container_id,
            package_count,
            names,
            name_hashes,
            package_ids,
            packages,
        })
    }
}

#[derive(Debug)]
struct FMinimalName {
    index: u32,
    number: i32,
}

impl Newable for FMinimalName {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            index: reader.read_u32::<LittleEndian>()?,
            number: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub struct FScriptObjectEntry {
    object_name: FMappedName,
    global_index: FPackageObjectIndex,
    outer_index: FPackageObjectIndex,
    cdo_class_index: FPackageObjectIndex,
}

impl Newable for FScriptObjectEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            object_name: FMappedName::new(reader)?,
            global_index: FPackageObjectIndex::new(reader)?,
            outer_index: FPackageObjectIndex::new(reader)?,
            cdo_class_index: FPackageObjectIndex::new(reader)?,
        })
    }
}

impl FScriptObjectEntry {
    pub fn get_outer_index(&self) -> &FPackageObjectIndex {
        &self.outer_index
    }

    pub fn get_object_name(&self) -> &FMappedName {
        &self.object_name
    }
}

#[derive(Debug)]
pub struct InitialLoadMetaData {
    script_objects: Vec<FScriptObjectEntry>,
}

impl InitialLoadMetaData {
    pub fn find_object(&self, index: &FPackageObjectIndex) -> Option<&FScriptObjectEntry> {
        self.script_objects.iter().find(|v| &v.global_index == index)
    }

    pub fn empty() -> Self {
        Self {
            script_objects: Vec::new(),
        }
    }

    pub fn get_package_name<'a>(&self, object: &FPackageObjectIndex, name_map: &'a FNameMap) -> Option<&'a str> {
        match self.find_object(object) {
            Some(obj) => match obj.object_name.get_name(name_map) {
                Ok(name) => Some(name),
                Err(_) => None,
            },
            None => None,
        }
    }
}

impl Newable for InitialLoadMetaData {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            script_objects: read_tarray(reader)?,
        })
    }
}

#[derive(Debug)]
pub struct FNameMap {
    names: Vec<String>,
}

impl FNameMap {
    pub fn empty() -> Self {
        Self {
            names: Vec::new(),
        }
    }

    pub fn from_strings(data: Vec<String>) -> Self {
        Self {
            names: data,
        }
    }

    pub fn get_name(&self, idx: usize) -> ParserResult<&str> {
        match &self.names.get(idx) {
            Some(data) => Ok(data),
            None => Err(ParserError::new(format!("Name not found at: {}", idx))),
        }
    }
}

impl Newable for FNameMap {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut names = Vec::new();

        loop {
            match read_short_string(reader) {
                Ok(data) => names.push(data),
                Err(_) => break,
            };
        }

        Ok(Self {
            names
        })
    }
}

#[derive(Debug)]
pub struct LoaderGlobalData {
    names: Arc<FNameMap>,
    imports: Arc<InitialLoadMetaData>,
}

impl LoaderGlobalData {
    pub fn empty() -> Self {
        Self {
            names: Arc::new(FNameMap::empty()),
            imports: Arc::new(InitialLoadMetaData::empty()),
        }
    }

    pub fn get_load_data(&self) -> Arc<InitialLoadMetaData> {
        Arc::clone(&self.imports)
    }

    pub fn get_name_map(&self) -> Arc<FNameMap> {
        Arc::clone(&self.names)
    }
}

#[derive(Debug)]
pub enum ChunkData {
    ContainerHeader(FContainerHeader),
    LoaderInitialLoadMeta(InitialLoadMetaData),
    LoaderGlobalNames(FNameMap),
}

pub struct UtocManager {
    chunk_ids: Vec<FIoChunkId>,
    offsets: Vec<FIoOffsetAndLength>,
    directory_index: FIoDirectoryIndexResource,
    file_list: Vec<String>,
    data: Arc<ReaderData>,
}

impl UtocManager {
    pub fn new(utoc: &[u8], key: Option<&str>) -> ParserResult<Self> {
        let mut reader = Cursor::new(utoc);
        let header = FIoStoreTocHeader::new(&mut reader)?;

        reader.seek(SeekFrom::Start(header.header_size as u64)).unwrap();

        let mut chunk_ids = Vec::new();
        for _i in 0..header.entry_count {
            chunk_ids.push(FIoChunkId::new(&mut reader)?);
        }

        let mut offsets = Vec::new();
        for _i in 0..header.entry_count {
            offsets.push(FIoOffsetAndLength::new(&mut reader)?);
        }

        let mut compressed_blocks = Vec::new();
        for _i in 0..header.compressed_block_entry_count {
            compressed_blocks.push(FIoStoreTocCompressedBlockEntry::new(&mut reader)?);
        }

        let mut compression_methods = Vec::new();
        for _i in 0..header.compression_method_name_count {
            let mut str_data = vec![0u8; header.compression_method_name_length as usize];
            reader.read_exact(&mut str_data)?;
            let mut str_content = std::str::from_utf8(&str_data).unwrap().to_owned();
            str_content.retain(|c| c != '\u{0}');
            compression_methods.push(str_content);
        }

        if header.is_signed() { // Signed
            let hash_size = reader.read_u32::<LittleEndian>()? as usize;
            let mut toc_hash = vec![0u8; hash_size];
            reader.read_exact(&mut toc_hash)?;

            let mut block_hash = vec![0u8; hash_size];
            reader.read_exact(&mut block_hash)?;

            let mut hashes = Vec::new();
            for _i in 0..header.compressed_block_entry_count {
                hashes.push(FSHAHash::new(&mut reader)?);
            }
        }

        let hex_key = if header.is_encrypted() {
            match key {
                Some(k) => Some(hex::decode(k).expect("Hex error")),
                None => return Err(ParserError::new(format!("No key provided"))),
            }
        } else { None };

        let (directory_index, file_list) = match header.directory_index_size > 0 {
            true => {
                let mut directory_buf = vec![0u8; header.directory_index_size as usize];
                reader.read_exact(&mut directory_buf)?;

                if header.is_encrypted() {
                    let decrypt = Ecb::<Aes256, ZeroPadding>::new_var((&hex_key).as_ref().unwrap(), Default::default()).unwrap();
                    decrypt.decrypt(&mut directory_buf).unwrap();
                }

                let mut directory_cursor = Cursor::new(directory_buf.as_slice());

                let index = FIoDirectoryIndexResource::new(&mut directory_cursor)?;
                let list = index.get_files(header.entry_count as usize);
                (index, list)
            },
            false => (FIoDirectoryIndexResource::empty(), Vec::new())
        };

        let data = Arc::new(ReaderData {
            compressed_blocks,
            compression_methods,
            header,
            key: hex_key,
        });

        Ok(Self {
            chunk_ids,
            offsets,
            directory_index,
            file_list,
            data,
        })
    }

    pub fn get_file_list(&self) -> &Vec<String> {
        &self.file_list
    }

    pub fn get_reader_data(&self) -> Arc<ReaderData> {
        Arc::clone(&self.data)
    }

    pub fn get_mount_point(&self) -> &str {
        &self.directory_index.mount_point
    }

    pub fn get_file(&self, file: &str) -> Option<&FIoOffsetAndLength> {
        for i in 0..self.file_list.len() {
            if file == self.file_list[i] {
                let chunk = &self.offsets[i];
                return Some(chunk);
            }
        }

        None
    }

    pub fn get_chunk_ids(&self) -> &Vec<FIoChunkId> {
        &self.chunk_ids
    }
}

pub struct Extractor {
    utoc: UtocManager,
    reader: UcasReader,
}

impl Extractor {
    pub fn new_header(asset_path: &str) -> ParserResult<FIoStoreTocHeader> {
        let utoc_path = asset_path.to_owned() + ".utoc";
        let mut file = File::open(utoc_path)?;
        let mut buffer = vec![0u8; 256];
        file.read_exact(&mut buffer)?;

        let mut reader = Cursor::new(buffer.as_slice());
        let header = FIoStoreTocHeader::new(&mut reader)?;

        Ok(header)
    }

    pub fn new(path: &str, key: Option<&str>) -> ParserResult<Self> {
        let utoc_path = path.to_owned() + ".utoc";
        let mut file = File::open(utoc_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let utoc = UtocManager::new(&buffer, key)?;        

        let ucas_reader = UcasReader::new(path, utoc.get_reader_data())?;

        Ok(Self {
            utoc,
            reader: ucas_reader,
        })
    }

    pub fn read_global(&mut self) -> ParserResult<LoaderGlobalData> {
        let initial_data = match self.read_chunk(0)? {
            ChunkData::LoaderInitialLoadMeta(d) => d,
            _ => return Err(ParserError::new(format!("Not a global chunk"))),
        };

        let name_map = match self.read_chunk(1)? {
            ChunkData::LoaderGlobalNames(d) => d,
            _ => return Err(ParserError::new(format!("Not a global chunk"))),
        };

        Ok(LoaderGlobalData {
            names: Arc::new(name_map),
            imports: Arc::new(initial_data),
        })
    }

    pub fn read_chunk(&mut self, idx: usize) -> ParserResult<ChunkData> {
        let chunk_offset = &self.utoc.offsets[idx];
        let chunk_id = &self.utoc.chunk_ids[idx];

        let mut chunk_data = vec![0u8; chunk_offset.length as usize];
        self.reader.seek(SeekFrom::Start(chunk_offset.offset))?;
        self.reader.read_exact(&mut chunk_data)?;

        let mut reader = Cursor::new(chunk_data.as_slice());

        match chunk_id.chunk_type {
            EIoChunkType::ContainerHeader => Ok(ChunkData::ContainerHeader(FContainerHeader::new(&mut reader)?)),
            EIoChunkType::LoaderInitialLoadMeta => Ok(ChunkData::LoaderInitialLoadMeta(InitialLoadMetaData::new(&mut reader)?)),
            EIoChunkType::LoaderGlobalNames => Ok(ChunkData::LoaderGlobalNames(FNameMap::new(&mut reader)?)),
            _ => Err(ParserError::new(format!("Type not supported: {:#?}", chunk_id))),
        }
    }

    pub fn get_file_list(&self) -> &Vec<String> {
        self.utoc.get_file_list()
    }

    pub fn get_mount_point(&self) -> &str {
        self.utoc.get_mount_point()
    }

    pub fn get_chunk_ids(&self) -> &Vec<FIoChunkId> {
        self.utoc.get_chunk_ids()
    }

    pub fn get_file(&mut self, file: &str) -> ParserResult<Vec<u8>> {
        for i in 0..self.utoc.file_list.len() {
            if file == self.utoc.file_list[i] {
                let chunk = &self.utoc.offsets[i];
                let mut data = vec![0u8; chunk.length as usize];

                self.reader.seek(SeekFrom::Start(chunk.offset))?;
                self.reader.read_exact(&mut data)?;

                return Ok(data);
            }
        }

        Err(ParserError::new(format!("Could not find file: {}", file)))
    }
}