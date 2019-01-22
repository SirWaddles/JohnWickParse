use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, BufReader, Seek, SeekFrom, Cursor};
use crate::assets::{FGuid, Newable, ReaderCursor, read_string, read_tarray};
use crate::rijndael;

const PAK_MAGIC: u32 = 0x5A6F12E1;
const PAK_SIZE: u32 = 8 + 16 + 20 + 1 + 16;

struct FPakInfo {
    encryption_key_guid: FGuid,
    encrypted_index: bool,
    version: u32,
    index_offset: u64,
    index_size: i64,
    index_hash: [u8; 20],
}

impl Newable for FPakInfo {
    fn new(reader: &mut ReaderCursor) -> Self {
        let encryption_key_guid = FGuid::new(reader);
        let encrypted_index = reader.read_u8().unwrap() != 0;
        let magic = reader.read_u32::<LittleEndian>().unwrap();

        if magic != PAK_MAGIC {
            panic!("Invalid pak file");
        }
        let version = reader.read_u32::<LittleEndian>().unwrap();
        let index_offset = reader.read_u64::<LittleEndian>().unwrap();
        let index_size = reader.read_i64::<LittleEndian>().unwrap();

        println!("index: {} {}", index_offset, index_size);
        let mut index_hash = [0u8; 20];
        reader.read_exact(&mut index_hash).unwrap();

        Self {
            encryption_key_guid,
            encrypted_index,
            version,
            index_offset,
            index_size,
            index_hash,
        }
    }
}

fn get_index(header: &FPakInfo, reader: &mut BufReader<File>) -> Vec<u8> {
    let mut ciphertext = vec![0u8; header.index_size as usize];
    reader.seek(SeekFrom::Start(header.index_offset)).unwrap();
    reader.read_exact(&mut ciphertext).unwrap();
    let key = hex::decode("265e1a5e2741895843d75728b73aeb6a814d3b0302fc69be39bb3f408b9b54e6").expect("Hex error");
    println!("Key: {}", key.len());
    rijndael::rijndael_decrypt_buf(&ciphertext, &key)
}

struct FPakCompressedBlock {
    compressed_start: i64,
    compressed_end: i64,
}

impl Newable for FPakCompressedBlock {
    fn new(reader: &mut ReaderCursor) -> Self {
        Self {
            compressed_start: reader.read_i64::<LittleEndian>().unwrap(),
            compressed_end: reader.read_i64::<LittleEndian>().unwrap(),
        }
    }
}

pub struct FPakEntry {
    filename: String,
    position: i64,
    size: i64,
    uncompressed_size: i64,
    compression_method: i32,
    hash: [u8; 20],
    compression_blocks: Vec<FPakCompressedBlock>,
    encrypted: bool,
    compression_block_size: u32,
    struct_size: u64,
}

impl FPakEntry {
    fn new(reader: &mut ReaderCursor, filename: String) -> Self {
        let seek_point = reader.position();
        let position = reader.read_i64::<LittleEndian>().unwrap();
        let size = reader.read_i64::<LittleEndian>().unwrap();
        let uncompressed_size = reader.read_i64::<LittleEndian>().unwrap();
        let compression_method = reader.read_i32::<LittleEndian>().unwrap();
        let mut hash = [0u8; 20];
        let mut compression_blocks: Vec<FPakCompressedBlock> = Vec::new();
        if compression_method != 0 {
            compression_blocks = read_tarray(reader);
        }
        reader.read_exact(&mut hash).unwrap();
        Self {
            filename, position, size, uncompressed_size, compression_method, hash, compression_blocks,
            encrypted: reader.read_u8().unwrap() != 0,
            compression_block_size: reader.read_u32::<LittleEndian>().unwrap(),
            struct_size: reader.position() - seek_point,
        }
    }

    pub fn get_filename(&self) -> &str {
        &self.filename[..]
    }
}

struct FPakIndex {
    mount_point: String,
    file_count: u32,
    index_entries: Vec<FPakEntry>,
}

impl FPakIndex {
    fn new(reader: &mut ReaderCursor) -> Self {
        let mount_point = read_string(reader);
        let file_count = reader.read_u32::<LittleEndian>().unwrap();
        println!("Reading {} files", file_count);
        let mut index_entries = Vec::new();
        for i in 0..file_count {
            println!("reading file: {}", i);
            let filename = read_string(reader);
            index_entries.push(FPakEntry::new(reader, filename));
        }
        
        Self {
            mount_point,
            file_count,
            index_entries,
        }
    }
}

pub struct PakExtractor {
    header: FPakInfo,
    index: FPakIndex,
}

impl PakExtractor {
    pub fn new(path: &str) -> Self {
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(-(PAK_SIZE as i64))).unwrap();
        let mut header_b = vec![0u8; PAK_SIZE as usize];
        reader.read_exact(&mut header_b).unwrap();

        let mut header_reader = Cursor::new(header_b);
        let header = FPakInfo::new(&mut header_reader);

        let index_data = get_index(&header, &mut reader);
        let mut index_reader = Cursor::new(index_data);
        let index = FPakIndex::new(&mut index_reader);

        Self {
            header,
            index,
        }
    }

    pub fn get_entries(&self) -> &Vec<FPakEntry> {
        &self.index.index_entries
    }
}