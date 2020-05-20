use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, BufReader, Seek, SeekFrom, Cursor};
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};
use aes_soft::Aes256;
use flate2::read::ZlibDecoder;
use crate::assets::{FGuid, Newable, ReaderCursor, read_string, read_tarray, ParserResult, ParserError};
use crate::decompress::oodle;

const PAK_MAGIC: u32 = 0x5A6F12E1;
const PAK_SIZE: u32 = 8 + 16 + 20 + 1 + 16 + (32 * 5);

#[allow(dead_code)]
#[derive(Debug)]
pub struct FPakInfo {
    encryption_key_guid: FGuid,
    encrypted_index: bool,
    version: u32,
    index_offset: u64,
    index_size: u64,
    index_hash: [u8; 20],
    compression_methods: Vec<String>,
}

#[allow(dead_code)]
impl FPakInfo {
    pub fn get_key_guid(&self) -> &FGuid {
        &self.encryption_key_guid
    }

    pub fn get_index_sizes(&self) -> (u64, u64) {
        (self.index_offset, self.index_size)
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
        let index_size = reader.read_u64::<LittleEndian>()?;

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

    let decrypt = Ecb::<Aes256, ZeroPadding>::new_var(&key, Default::default()).unwrap();
    decrypt.decrypt(&mut ciphertext).unwrap();
    ciphertext
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
    pub position: i64,
    pub size: u64,
    uncompressed_size: u64,
    compression_method: u32,
    compression_blocks: Vec<FPakCompressedBlock>,
    pub hash: [u8; 20],
    pub encrypted: bool,
    compression_block_size: u32,
    pub struct_size: u64,
}

#[allow(dead_code)]
impl FPakEntry {
    fn new(reader: &mut ReaderCursor, filename: String) -> ParserResult<Self> {
        let seek_point = reader.position();
        let position = reader.read_i64::<LittleEndian>()?;
        let size = reader.read_u64::<LittleEndian>()?;
        let uncompressed_size = reader.read_u64::<LittleEndian>()?;
        let compression_method = reader.read_u32::<LittleEndian>()?;
        let mut hash = [0u8; 20];
        reader.read_exact(&mut hash)?;
        let mut compression_blocks: Vec<FPakCompressedBlock> = Vec::new();
        if compression_method != 0 {
            compression_blocks = read_tarray(reader)?;
        }

        Ok(Self {
            filename, position, size, uncompressed_size, compression_method, compression_blocks, hash,
            encrypted: reader.read_u8()? != 0,
            compression_block_size: reader.read_u32::<LittleEndian>()?,
            struct_size: reader.position() - seek_point,
        })
    }

    /// Gets the internal filename for the file this represents
    pub fn get_filename(&self) -> &str {
        &self.filename[..]
    }

    fn from_encoded(reader: &mut ReaderCursor, dir_name: &str, index_data: &FPathHashIndexEntry) -> ParserResult<Self> {
        reader.seek(SeekFrom::Start(index_data.location as u64))?;
        let flags = reader.read_u32::<LittleEndian>()?;

        let offset_safe = (flags & (1 << 31)) != 0;
        let position = match offset_safe {
            true => reader.read_u32::<LittleEndian>()? as i64,
            false => reader.read_u64::<LittleEndian>()? as i64,
        };

        let uncompressed_size_safe = (flags & (1 << 30)) != 0;
        let uncompressed_size = match uncompressed_size_safe {
            true => reader.read_u32::<LittleEndian>()? as u64,
            false => reader.read_u64::<LittleEndian>()?,
        };

        let mut size = uncompressed_size;
        let compression_method = (flags >> 23) & 0x3f;
        if compression_method != 0 {
            let size_safe = (flags & (1 << 29)) != 0;
            size = match size_safe {
                true => reader.read_u32::<LittleEndian>()? as u64,
                false => reader.read_u64::<LittleEndian>()?,
            };
        }

        let encrypted = (flags & (1 << 22)) != 0;

        let compression_block_count = (flags >> 6) & 0xffff;
        let mut compression_block_size = 0;
        if compression_block_count > 0 {
            compression_block_size = match uncompressed_size < 65536 {
                true => uncompressed_size as u32,
                false => ((flags & 0x3f) << 11),
            };
        }

        let mut struct_size: u64 = 8 + 8 + 8 + 20 + 4 + 4 + 1;

        if compression_method != 0 {
            struct_size += 4 + (compression_block_count as u64) * 16;
        }

        let mut compression_blocks = Vec::new();
        let mut compressed_block_offset = struct_size as i64;

        if compression_block_count == 1 {
            compression_blocks.push(FPakCompressedBlock {
                compressed_start: struct_size as i64,
                compressed_end: (struct_size + size) as i64,
            });
        } else {
            for _i in 0..compression_block_count {
                let size_data = reader.read_u32::<LittleEndian>()? as i64;
                compression_blocks.push(FPakCompressedBlock {
                    compressed_start: compressed_block_offset,
                    compressed_end: compressed_block_offset + size_data,
                });
                compressed_block_offset += size_data;
            }
        }

        Ok(Self {
            filename: dir_name.to_owned() + &index_data.key,
            position,
            size,
            uncompressed_size,
            compression_method,
            encrypted,
            // Not gonna support this straight away
            compression_blocks,
            hash: [0u8; 20],
            compression_block_size,
            struct_size,
        })
    }
}

#[allow(dead_code)]
pub struct FPakIndex {
    mount_point: String,
    file_count: u32,
    index_entries: Vec<FPakEntry>,
    encoded_pak: Vec<u8>,
    path_index: (i64, i64),
    dir_index: (i64, i64),
}

impl Newable for FPakIndex {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mount_point = read_string(reader)?;
        if mount_point.len() > 1024 {
            return Err(ParserError::new(format!("Could not read Pak Archive")));
        }
        let file_count = reader.read_u32::<LittleEndian>()?;

        let _hash_seed = reader.read_u64::<LittleEndian>()?;
        let has_path_index = reader.read_i32::<LittleEndian>()? != 0;
        let path_index = match has_path_index {
            true => {
                let path_index_offset = reader.read_i64::<LittleEndian>()?;
                let path_index_size = reader.read_i64::<LittleEndian>()?;
                let mut hash = [0u8; 20];
                reader.read_exact(&mut hash)?;
                (path_index_offset, path_index_size)
            },
            false => return Err(ParserError::new(format!("No path index present"))),
        };

        let has_dir_index = reader.read_i32::<LittleEndian>()? != 0;
        let dir_index = match has_dir_index {
            true => {
                let dir_index_offset = reader.read_i64::<LittleEndian>()?;
                let dir_index_size = reader.read_i64::<LittleEndian>()?;
                let mut hash = [0u8; 20];
                reader.read_exact(&mut hash)?;
                (dir_index_offset, dir_index_size)
            },
            false => return Err(ParserError::new(format!("No directory index present"))),
        };

        let encoded_pak: Vec<u8> = read_tarray(reader)?;
        let new_file_count = reader.read_u32::<LittleEndian>()?;

        let mut index_entries = Vec::new();
        for _i in 0..new_file_count {
            index_entries.push(FPakEntry::new(reader, "test".to_owned())?);
        }

        Ok(Self {
            mount_point,
            file_count,
            index_entries,
            encoded_pak,
            path_index,
            dir_index,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FPathHashIndexEntry {
    key: String,
    location: u32,
}

impl Newable for FPathHashIndexEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            key: read_string(reader)?,
            location: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FPakDirectoryEntry {
    key: String,
    value: Vec<FPathHashIndexEntry>,
}

impl Newable for FPakDirectoryEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            key: read_string(reader)?,
            value: read_tarray(reader)?,
        })
    }
}

impl FPakIndex {
    pub fn get_entries(&self) -> &[FPakEntry] {
        self.index_entries.as_slice()
    }

    pub fn get_mount_point(&self) -> &str {
        &self.mount_point
    }

    pub fn get_dir_index(&self) -> (i64, i64) {
        self.dir_index
    }

    pub fn update_from_index(&mut self, reader: &mut ReaderCursor) -> ParserResult<()> {
        let directory_index: Vec<FPakDirectoryEntry> = read_tarray(reader)?;
        let mut encoded_cursor = Cursor::new(self.encoded_pak.as_slice());
        let mut pak_entries = Vec::new();
        for dir_entry in &directory_index {
            for pak_entry in &dir_entry.value {
                pak_entries.push(FPakEntry::from_encoded(&mut encoded_cursor, &dir_entry.key, &pak_entry)?);
            }
        }

        self.index_entries = pak_entries;

        Ok(())
    }
}

/// PakExtractor can read the contents of a `.pak` file
#[allow(dead_code)]
pub struct PakExtractor {
    header: FPakInfo,
    index: FPakIndex,
    key: Vec<u8>,
    reader: BufReader<File>,
}

fn decompress_block(input: &[u8], output_size: u64, compression_method: &str) -> Vec<u8> {
    match compression_method {
        "Oodle" => oodle::decompress_stream(output_size, input).unwrap(),
        "Zlib" => decompress_zlib(output_size, input),
        _ => Vec::new(),
    }
}

fn decompress_zlib(output_size: u64, input: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; output_size as usize];
    let mut z = ZlibDecoder::new(input);
    z.read_exact(&mut data).unwrap();
    data
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

        let mut header_reader = Cursor::new(header_b.as_slice());
        let header = FPakInfo::new(&mut header_reader)?;

        let index_data = get_index(&header, &mut reader, key);
        let mut index_reader = Cursor::new(index_data.as_slice());
        let mut index = FPakIndex::new(&mut index_reader)?;

        // read directory index
        reader.seek(SeekFrom::Start(index.dir_index.0 as u64))?;
        let mut dir_index_b = vec![0u8; index.dir_index.1 as usize];
        reader.read_exact(&mut dir_index_b)?;

        let key = hex::decode(key).expect("Hex error");
        let decrypt = Ecb::<Aes256, ZeroPadding>::new_var(&key, Default::default()).unwrap();
        decrypt.decrypt(&mut dir_index_b).unwrap();
        let mut directory_reader = Cursor::new(dir_index_b.as_slice());

        index.update_from_index(&mut directory_reader)?;

        Ok(Self {
            header,
            index,
            key,
            reader,
        })
    }

    pub fn new_header(path: &str) -> ParserResult<FPakInfo> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(-(PAK_SIZE as i64)))?;
        let mut header_b = vec![0u8; PAK_SIZE as usize];
        reader.read_exact(&mut header_b)?;

        let mut header_reader = Cursor::new(header_b.as_slice());
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

            let decrypt = Ecb::<Aes256, ZeroPadding>::new_var(&self.key, Default::default()).unwrap();
            decrypt.decrypt(&mut enc_buffer).unwrap();

            buffer.copy_from_slice(&enc_buffer[..file.size as usize]);
        } else {
            self.reader.read_exact(&mut buffer).unwrap();
        }

        if file.compression_method != 0 {
            let mut decompressed_buffer = Vec::new();
            let compression_method = &self.header.compression_methods[(file.compression_method - 1) as usize];

            for block in &file.compression_blocks {
                let block_buffer = &buffer[((block.compressed_start - file.struct_size as i64) as usize)..((block.compressed_end - file.struct_size as i64) as usize)];
                let result_size = std::cmp::min(file.compression_block_size as u64, file.uncompressed_size - decompressed_buffer.len() as u64);
                let decompressed_block = decompress_block(block_buffer, result_size, compression_method);
                decompressed_buffer.extend_from_slice(&decompressed_block);
            }

            buffer = decompressed_buffer;
        }

        buffer
    }

    pub fn get_mount_point(&self) -> &str {
        &self.index.mount_point
    }
}
