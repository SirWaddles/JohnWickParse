use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Seek, SeekFrom};
use super::*;

const LOCRES_MAGIC: FGuid = FGuid {a: 0x7574140E, b: 0xFC034A67, c: 0x9D90154A, d: 0x1B7F37C3};
const INDEX_NONE: i64 = -1;

#[derive(Debug, Serialize)]
struct FTextKey {
    str_hash: u32,
    text: String,
}

impl Newable for FTextKey {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            str_hash: reader.read_u32::<LittleEndian>()?,
            text: read_string(reader)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FTextLocalizationResourceString {
	data: String,
	ref_count: i32,
}

impl Newable for FTextLocalizationResourceString {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            data: read_string(reader)?,
            ref_count: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FEntry {
    key: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct LocaleNamespace {
    namespace: String,
    data: Vec<FEntry>,
}

#[derive(Debug, Serialize)]
pub struct FTextLocalizationResource {
    version: u8,
    string_data: Vec<LocaleNamespace>,
}

impl FTextLocalizationResource {
    pub fn from_buffer(locres: &[u8]) -> ParserResult<Self> {
        let mut reader = ReaderCursor::new(locres);
        let magic = FGuid::new(&mut reader)?;

        if magic != LOCRES_MAGIC {
            return Err(ParserError::new(format!("Wrong magic Guid")));
        }

        let version = reader.read_u8()?;

        let str_array_offset = reader.read_i64::<LittleEndian>()?;
        if str_array_offset == INDEX_NONE {
            return Err(ParserError::new(format!("No offset found")));
        }

        // only works for version: optimized
        let current_offset = reader.position();
        reader.seek(SeekFrom::Start(str_array_offset as u64))?;
        let localized_strings: Vec<FTextLocalizationResourceString> = read_tarray(&mut reader)?;
        reader.seek(SeekFrom::Start(current_offset))?;

        let _entry_count = reader.read_u32::<LittleEndian>()?;
        let namespace_count = reader.read_u32::<LittleEndian>()?;
        let mut string_data = Vec::new();
        for _i in 0..namespace_count {
            let namespace = FTextKey::new(&mut reader)?;
            let key_count = reader.read_u32::<LittleEndian>()?;

            let mut strings = Vec::new();

            for _j in 0..key_count {
                let text_key = FTextKey::new(&mut reader)?;
                let _source_hash = reader.read_u32::<LittleEndian>()?;
                let string_index = reader.read_i32::<LittleEndian>()?;
                if string_index >= 0 && string_index < localized_strings.len() as i32 {
                    strings.push(FEntry {
                        key: text_key.text,
                        data: localized_strings[string_index as usize].data.clone(),
                    });
                }
            }

            string_data.push(LocaleNamespace {
                namespace: namespace.text,
                data: strings,
            });
        }

        Ok(Self {
            version, string_data
        })
    }
}