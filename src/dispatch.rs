use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, BufReader, Seek, SeekFrom, Cursor};
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};
use aes_soft::Aes256;
use flate2::read::ZlibDecoder;
use crate::assets::{FGuid, Newable, ReaderCursor, read_string, read_tarray, ParserResult, ParserError};

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

#[derive(Debug)]
struct FIoStoreTocHeader {
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
struct FIoChunkId {
    data: [u8; 12],
}

impl Newable for FIoChunkId {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut data = [0u8; 12];
        reader.read_exact(&mut data)?;

        Ok(Self {
            data,
        })
    }
}

#[derive(Debug)]
struct FIoOffsetAndLength {
    data: [u8; 10],
}

impl Newable for FIoOffsetAndLength {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut data = [0u8; 10];
        reader.read_exact(&mut data)?;

        Ok(Self {
            data,
        })
    }
}

#[derive(Debug)]
struct FIoStoreTocCompressedBlockEntry {
    data: [u8; 12],
}

impl Newable for FIoStoreTocCompressedBlockEntry {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let mut data = [0u8; 12];
        reader.read_exact(&mut data)?;

        Ok(Self {
            data,
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

pub struct Extractor {
    header: FIoStoreTocHeader,
    chunk_ids: Vec<FIoChunkId>,
    offsets: Vec<FIoOffsetAndLength>,
    compressed_blocks: Vec<FIoStoreTocCompressedBlockEntry>,
    compression_methods: Vec<String>,
    directory_index: FIoDirectoryIndexResource,
}

impl Extractor {
    pub fn new(path: &str, key: &str) -> ParserResult<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer);

        let mut reader = Cursor::new(buffer.as_slice());
        let header = FIoStoreTocHeader::new(&mut reader)?;

        reader.seek(SeekFrom::Start(header.header_size as u64)).unwrap();

        println!("{:#?}", header);

        let mut chunk_ids = Vec::new();
        for _i in 0..header.entry_count {
            chunk_ids.push(FIoChunkId::new(&mut reader)?);
        }

        println!("After IDs: {}", reader.position());

        let mut offsets = Vec::new();
        for _i in 0..header.entry_count {
            offsets.push(FIoOffsetAndLength::new(&mut reader)?);
        }

        println!("After Offsets: {}", reader.position());

        let mut compressed_blocks = Vec::new();
        for _i in 0..header.compressed_block_entry_count {
            compressed_blocks.push(FIoStoreTocCompressedBlockEntry::new(&mut reader)?);
        }

        println!("After Blocks: {}", reader.position());

        let mut compression_methods = Vec::new();
        for _i in 0..header.compression_method_name_count {
            let mut str_data = vec![0u8; header.compression_method_name_length as usize];
            reader.read_exact(&mut str_data)?;
            let mut str_content = std::str::from_utf8(&str_data).unwrap().to_owned();
            str_content.retain(|c| c != '\u{0}');
            compression_methods.push(str_content);
        }

        println!("After Methods: {}", reader.position());

        if header.container_flags & (1 << 2) != 0 { // Signed
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

        let directory_index = FIoDirectoryIndexResource::new(&mut reader)?;

        Ok(Self {
            header,
            chunk_ids,
            offsets,
            compressed_blocks,
            compression_methods,
            directory_index,
        })
    }
}