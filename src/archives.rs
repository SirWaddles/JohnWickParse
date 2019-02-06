use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, BufReader, Seek, SeekFrom, Cursor};
use crate::assets::{FGuid, Newable, ReaderCursor, read_string, read_tarray, ParserResult};
use crate::rijndael;

const PAK_MAGIC: u32 = 0x5A6F12E1;
const PAK_SIZE: u32 = 8 + 16 + 20 + 1 + 16;

#[allow(dead_code)]
struct FPakInfo {
    encryption_key_guid: FGuid,
    encrypted_index: bool,
    version: u32,
    index_offset: u64,
    index_size: i64,
    index_hash: [u8; 20],
}

impl Newable for FPakInfo {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let encryption_key_guid = FGuid::new(reader)?;
        let encrypted_index = reader.read_u8()? != 0;
        let magic = reader.read_u32::<LittleEndian>()?;

        if magic != PAK_MAGIC {
            panic!("Invalid pak file");
        }
        let version = reader.read_u32::<LittleEndian>()?;
        let index_offset = reader.read_u64::<LittleEndian>()?;
        let index_size = reader.read_i64::<LittleEndian>()?;

        println!("index: {} {}", index_offset, index_size);
        let mut index_hash = [0u8; 20];
        reader.read_exact(&mut index_hash)?;

        Ok(Self {
            encryption_key_guid,
            encrypted_index,
            version,
            index_offset,
            index_size,
            index_hash,
        })
    }
}

fn get_index(header: &FPakInfo, reader: &mut BufReader<File>, key: &str) -> Vec<u8> {
    let mut ciphertext = vec![0u8; header.index_size as usize];
    reader.seek(SeekFrom::Start(header.index_offset)).unwrap();
    reader.read_exact(&mut ciphertext).unwrap();
    let key = hex::decode(key).expect("Hex error");
    println!("Key: {}", key.len());
    rijndael::rijndael_decrypt_buf(&ciphertext, &key)
}

#[allow(dead_code)]
#[derive(Clone)]
struct FPakCompressedBlock {
    compressed_start: i64,
    compressed_end: i64,
}

impl Newable for FPakCompressedBlock {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            compressed_start: reader.read_i64::<LittleEndian>()?,
            compressed_end: reader.read_i64::<LittleEndian>()?,
        })
    }
}

/// Contains the details of a file residing in a `.pak` file
#[allow(dead_code)]
#[derive(Clone)]
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

#[allow(dead_code)]
impl FPakEntry {
    fn new(reader: &mut ReaderCursor, filename: String) -> ParserResult<Self> {
        let seek_point = reader.position();
        let position = reader.read_i64::<LittleEndian>()?;
        let size = reader.read_i64::<LittleEndian>()?;
        let uncompressed_size = reader.read_i64::<LittleEndian>()?;
        let compression_method = reader.read_i32::<LittleEndian>()?;
        let mut hash = [0u8; 20];
        let mut compression_blocks: Vec<FPakCompressedBlock> = Vec::new();
        if compression_method != 0 {
            compression_blocks = read_tarray(reader)?;
        }
        reader.read_exact(&mut hash)?;
        Ok(Self {
            filename, position, size, uncompressed_size, compression_method, hash, compression_blocks,
            encrypted: reader.read_u8()? != 0,
            compression_block_size: reader.read_u32::<LittleEndian>()?,
            struct_size: reader.position() - seek_point,
        })
    }

    /// Gets the internal filename for the file this represents
    pub fn get_filename(&self) -> &str {
        &self.filename[..]
    }
}

#[allow(dead_code)]
struct FPakIndex {
    mount_point: String,
    file_count: u32,
    index_entries: Vec<FPakEntry>,
}

impl FPakIndex {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mount_point = read_string(reader)?;
        let file_count = reader.read_u32::<LittleEndian>()?;
        println!("Reading {} files", file_count);
        let mut index_entries = Vec::new();
        for _i in 0..file_count {
            let filename = read_string(reader)?;
            index_entries.push(FPakEntry::new(reader, filename)?);
        }
        
        Ok(Self {
            mount_point,
            file_count,
            index_entries,
        })
    }
}

/// PakExtractor can read the contents of a `.pak` file
#[allow(dead_code)]
pub struct PakExtractor {
    header: FPakInfo,
    index: FPakIndex,
    key: String,
    reader: BufReader<File>,
}

#[allow(dead_code)]
impl PakExtractor {
    /// Create a `PakExtractor` by specifying the path to the pak file on disk, and the encryption key to the file index
    pub fn new(path: &str, key: &str) -> ParserResult<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(-(PAK_SIZE as i64)))?;
        let mut header_b = vec![0u8; PAK_SIZE as usize];
        reader.read_exact(&mut header_b)?;

        let mut header_reader = Cursor::new(header_b);
        let header = FPakInfo::new(&mut header_reader)?;

        let index_data = get_index(&header, &mut reader, key);
        let mut index_reader = Cursor::new(index_data);
        let index = FPakIndex::new(&mut index_reader)?;

        Ok(Self {
            header,
            index,
            key: key.to_owned(),
            reader,
        })
    }

    /// Get a list of `FPakEntry` that can be used with `get_file` to extract files.
    pub fn get_entries(&self) -> &Vec<FPakEntry> {
        &self.index.index_entries
    }

    /// Uses an `FPakEntry` to seek to and extract a file from a `.pak` file
    /// 
    /// Note that the `FPakEntry` must come from the same `PakExtractor`, using a mismatched one will panic
    pub fn get_file(&mut self, file: &FPakEntry) -> Vec<u8> {
        let start_pos = file.position as u64 + file.struct_size;
        self.reader.seek(SeekFrom::Start(start_pos)).unwrap();
        let mut buffer = vec![0u8; file.size as usize];

        if file.encrypted {
            let enc_size = match file.size % 16 {
                0 => file.size,
                _ => ((file.size / 16) + 1) * 16,
            };
            let mut enc_buffer = vec![0u8; enc_size as usize];
            self.reader.read_exact(&mut enc_buffer).unwrap();
            let key = hex::decode(&self.key).expect("Hex error");
            let plain_buffer = rijndael::rijndael_decrypt_buf(&enc_buffer, &key);
            let mut plain_cursor = Cursor::new(plain_buffer);
            plain_cursor.read_exact(&mut buffer).unwrap();
        } else {
            self.reader.read_exact(&mut buffer).unwrap();
        }

        buffer
    }
}