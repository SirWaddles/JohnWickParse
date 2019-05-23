use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, BufReader, Seek, SeekFrom, Cursor};
use crate::assets::{FGuid, Newable, ReaderCursor, read_string, read_tarray, ParserResult, ParserError};
use crate::rijndael;
use crate::decompress::oodle;

const PAK_MAGIC: u32 = 0x5A6F12E1;
const PAK_SIZE: u32 = 8 + 16 + 20 + 1 + 16 + (32 * 5);

#[allow(dead_code)]
pub struct FPakInfo {
    encryption_key_guid: FGuid,
    encrypted_index: bool,
    version: u32,
    index_offset: u64,
    index_size: i64,
    index_hash: [u8; 20],
    compression_methods: Vec<String>,
}

#[allow(dead_code)]
impl FPakInfo {
    pub fn get_key_guid(&self) -> &FGuid {
        &self.encryption_key_guid
    }
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

        let mut index_hash = [0u8; 20];
        reader.read_exact(&mut index_hash)?;

        let mut compression_methods = Vec::new();

        for _i in 0..5 {
            let mut bytes = [0u8; 32];
            reader.read_exact(&mut bytes)?;
            let mut length = 32;
            for p in 0..32 {
                if bytes[p] == 0 {
                    length = p;
                    break;
                }
            };

            let fstr = std::str::from_utf8(&bytes[0..length])?.to_owned();
            compression_methods.push(fstr);
        }

        Ok(Self {
            encryption_key_guid,
            encrypted_index,
            version,
            index_offset,
            index_size,
            index_hash,
            compression_methods,
        })
    }
}

fn get_index(header: &FPakInfo, reader: &mut BufReader<File>, key: &str) -> Vec<u8> {
    let mut ciphertext = vec![0u8; header.index_size as usize];
    reader.seek(SeekFrom::Start(header.index_offset)).unwrap();
    reader.read_exact(&mut ciphertext).unwrap();
    if !header.encrypted_index {
        return ciphertext;
    }
    let key = hex::decode(key).expect("Hex error");
    rijndael::rijndael_decrypt_buf(&ciphertext, &key)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct FPakEntry {
    filename: String,
    position: i64,
    size: i64,
    uncompressed_size: u64,
    compression_method: u32,
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
        let uncompressed_size = reader.read_u64::<LittleEndian>()?;
        let compression_method = reader.read_u32::<LittleEndian>()?;
        let mut hash = [0u8; 20];
        reader.read_exact(&mut hash)?;
        let mut compression_blocks: Vec<FPakCompressedBlock> = Vec::new();
        if compression_method != 0 {
            compression_blocks = read_tarray(reader)?;
        }
        
        Ok(Self {
            filename, position, size, uncompressed_size, compression_method, compression_blocks,
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
        if mount_point.len() > 1024 {
            return Err(ParserError::new(format!("Could not read Pak Archive")));
        }
        let file_count = reader.read_u32::<LittleEndian>()?;
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

    pub fn new_header(path: &str) -> ParserResult<FPakInfo> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(-(PAK_SIZE as i64)))?;
        let mut header_b = vec![0u8; PAK_SIZE as usize];
        reader.read_exact(&mut header_b)?;

        let mut header_reader = Cursor::new(header_b);
        let header = FPakInfo::new(&mut header_reader)?;

        Ok(header)
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

        if file.compression_method != 0 {
            let compression_method = &self.header.compression_methods[(file.compression_method - 1) as usize];
            if compression_method == "Oodle" {
                let mut compressed_buffer = buffer.clone();
                let uncompressed = oodle::decompress_stream(file.uncompressed_size, &mut compressed_buffer).unwrap();
                buffer = uncompressed;
            }
        }

        buffer
    }

    pub fn get_mount_point(&self) -> &str {
        &self.index.mount_point
    }
}