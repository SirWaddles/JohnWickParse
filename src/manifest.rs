use std::fs::File;
use std::io::{Read, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use crate::assets::{Newable, ReaderCursor, read_string, read_tarray, ParserResult, ParserError, FGuid};

const HEADER_MAGIC: u32 = 0x44BEC00C;

#[derive(Debug)]
struct FManifestHeader {
    header_size: u32,
    data_size_uncompressed: u32,
    data_size_compressed: u32,
    hash: [u8; 20],
    stored_as: u8,
    version: u32,
}

impl Newable for FManifestHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let magic = reader.read_u32::<LittleEndian>()?;
        if magic != HEADER_MAGIC {
            return Err(ParserError::new(format!("Manifest not valid")));
        }

        let header_size = reader.read_u32::<LittleEndian>()?;
        let data_size_uncompressed = reader.read_u32::<LittleEndian>()?;
        let data_size_compressed = reader.read_u32::<LittleEndian>()?;
        let mut hash = [0u8; 20];
        reader.read_exact(&mut hash)?;

        let stored_as = reader.read_u8()?;
        let version = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            header_size, data_size_compressed, data_size_uncompressed,
            hash, stored_as, version,
        })
    }
}

#[derive(Debug)]
struct FManifestMeta {
    data_size: u32,
    data_version: u8,
    feature_level: u32,
    file_data: bool,
    app_id: u32,
    app_name: String,
    build_version: String,
    launch_exe: String,
    launch_command: String,
    prereq_ids: Vec<String>,
    prereq_name: String,
    prereq_path: String,
    prereq_args: String,
}

impl Newable for FManifestMeta {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {        
        Ok(Self {
            data_size: reader.read_u32::<LittleEndian>()?,
            data_version: reader.read_u8()?,
            feature_level: reader.read_u32::<LittleEndian>()?,
            file_data: reader.read_u8()? != 0,
            app_id: reader.read_u32::<LittleEndian>()?,
            app_name: read_string(reader)?,
            build_version: read_string(reader)?,
            launch_exe: read_string(reader)?,
            launch_command: read_string(reader)?,
            prereq_ids: read_tarray(reader)?,
            prereq_name: read_string(reader)?,
            prereq_path: read_string(reader)?,
            prereq_args: read_string(reader)?,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct FChunkInfo {
    pub guid: FGuid,
    hash: u64,
    sha_hash: [u8; 20],
    pub group_number: u8,
    pub window_size: u32,
    pub file_size: i64,
}

#[derive(Debug)]
struct FChunkDataList {
    chunks: Vec<FChunkInfo>,
}

impl Newable for FChunkDataList {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {    
        let _data_size = reader.read_u32::<LittleEndian>()?;
        let _data_version = reader.read_u8()?;
        let element_count = reader.read_u32::<LittleEndian>()?;

        let mut chunks: Vec<FChunkInfo> = Vec::new();
        chunks.resize(element_count as usize, Default::default());

        for chunk in chunks.as_mut_slice() { chunk.guid = FGuid::new(reader)?; }
        for chunk in chunks.as_mut_slice() { chunk.hash = reader.read_u64::<LittleEndian>()?; }
        for chunk in chunks.as_mut_slice() { 
            let mut sha_hash = [0u8; 20];
            reader.read_exact(&mut sha_hash)?;
            chunk.sha_hash = sha_hash;
        }
        for chunk in chunks.as_mut_slice() { chunk.group_number = reader.read_u8()?; }
        for chunk in chunks.as_mut_slice() { chunk.window_size = reader.read_u32::<LittleEndian>()?; }
        for chunk in chunks.as_mut_slice() { chunk.file_size = reader.read_i64::<LittleEndian>()?; }

        Ok(Self {
            chunks,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct FChunkPart {
    pub guid: FGuid,
    pub offset: u32,
    pub size: u32,
}

impl Newable for FChunkPart {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let _data_size = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            guid: FGuid::new(reader)?,
            offset: reader.read_u32::<LittleEndian>()?,
            size: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct FFileManifest {
    pub filename: String,
    symlink_target: String,
    sha_hash: [u8; 20],
    meta_flags: u8,
    pub install_tags: Vec<String>,
    pub chunk_parts: Vec<FChunkPart>,
    pub file_size: u64,
}

impl Newable for FFileManifestList {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let _data_size = reader.read_u32::<LittleEndian>()?;
        let _data_version = reader.read_u8()?;
        let element_count = reader.read_u32::<LittleEndian>()?;

        let mut files: Vec<FFileManifest> = Vec::new();
        files.resize(element_count as usize, Default::default());

        for file in files.as_mut_slice() { file.filename = read_string(reader)?; }
        for file in files.as_mut_slice() { file.symlink_target = read_string(reader)?; }
        for file in files.as_mut_slice() { 
            let mut sha_hash = [0u8; 20];
            reader.read_exact(&mut sha_hash)?;
            file.sha_hash = sha_hash;
        }
        for file in files.as_mut_slice() { file.meta_flags = reader.read_u8()?; }
        for file in files.as_mut_slice() { file.install_tags = read_tarray(reader)?; }
        for file in files.as_mut_slice() { file.chunk_parts = read_tarray(reader)?; }

        Ok(Self {
            file_list: files,
        })
    }
}

#[derive(Debug)]
struct FFileManifestList {
    file_list: Vec<FFileManifest>,
}

pub struct Manifest {
    header: FManifestHeader,
    meta: FManifestMeta,
    chunk_list: FChunkDataList,
    file_list: FFileManifestList,
}

impl Manifest {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let header = FManifestHeader::new(reader)?;
        
        let mut input_buffer = vec![0u8; header.data_size_compressed as usize];
        reader.read_exact(&mut input_buffer)?;
        let mut output_buffer = vec![0u8; header.data_size_uncompressed as usize];
        let mut decoder = ZlibDecoder::new(input_buffer.as_slice());
        decoder.read_exact(&mut output_buffer)?;

        let mut input_cursor = Cursor::new(output_buffer.as_slice());
        let manifest_meta = FManifestMeta::new(&mut input_cursor)?;
        let chunk_list = FChunkDataList::new(&mut input_cursor)?;
        let manifest_list = FFileManifestList::new(&mut input_cursor)?;

        Ok(Self {
            header,
            meta: manifest_meta,
            chunk_list,
            file_list: manifest_list,
        })
    }

    pub fn from_buffer(buffer: &[u8]) -> ParserResult<Self> {
        let mut cursor = Cursor::new(buffer);
        Self::new(&mut cursor)
    }

    pub fn from_file(path: &str) -> ParserResult<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut reader = Cursor::new(buffer.as_slice());
        Self::new(&mut reader)
    }

    pub fn get_chunks(&self) -> &Vec<FChunkInfo> {
        &self.chunk_list.chunks
    }

    pub fn get_files(&self) -> &Vec<FFileManifest> {
        &self.file_list.file_list
    }
}