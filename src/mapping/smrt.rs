use std::io::{Read, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use crate::mapping::{ClassMapping, EnumMapping, PropertyMapping, TagMapping};
use crate::assets::{Newable, ReaderCursor, ParserResult, ParserError};
use crate::decompress::oodle;

const USMAP_MAGIC: u16 = 0x30C4;
const USMAP_HEADER_SIZE: usize = 12;

enum CompressionMethod {
    None,
    Oodle,
    Brotli,
}

struct UsmapHeader {
    version: u8,
    method: CompressionMethod, 
    comp_size: u32,
    size: u32,
}

impl Newable for UsmapHeader {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        let magic = reader.read_u16::<LittleEndian>()?;
        if magic != USMAP_MAGIC {
            return Err(ParserError::new(format!("Magic does not match usmap.")));
        }

        let version = reader.read_u8()?;
        let method = reader.read_u8()?;
        let method = match method {
            0 => CompressionMethod::None,
            1 => CompressionMethod::Oodle,
            2 => CompressionMethod::Brotli,
            _ => return Err(ParserError::new(format!("Unknown compression method"))),
        };

        let comp_size = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            version, method, comp_size, size,
        })
    }
}

fn get_usmap_data(mut data: Vec<u8>, header: &UsmapHeader) -> ParserResult<Vec<u8>> {
    data.drain(0..USMAP_HEADER_SIZE);
    match header.method {
        CompressionMethod::None => {
            Ok(data)
        },
        CompressionMethod::Oodle => {
            Ok(oodle::decompress_stream(header.size as u64, data.as_slice())?)
        },
        _ => return Err(ParserError::new(format!("Unsupported Compression Method"))),
    }
}

fn read_usmap_string(reader: &mut ReaderCursor) -> ParserResult<String> {
    let name_length = reader.read_u8()?;
    let mut bytes = vec![0u8; name_length as usize];
    reader.read_exact(&mut bytes)?;
    Ok(std::str::from_utf8(&bytes)?.to_owned())
}

fn read_usmap_name(reader: &mut ReaderCursor, name_map: &Vec<String>) -> ParserResult<String> {
    let name_idx = reader.read_u32::<LittleEndian>()?;
    if name_idx == u32::MAX {
        return Ok("".to_owned());
    }
    Ok(name_map[name_idx as usize].clone())
}

fn read_usmap_prop(reader: &mut ReaderCursor, name_map: &Vec<String>) -> ParserResult<TagMapping> {
    Ok(match reader.read_u8()? {
        0 => TagMapping::ByteProperty,
		1 => TagMapping::BoolProperty,
		2 => TagMapping::IntProperty,
		3 => TagMapping::FloatProperty,
		4 => TagMapping::ObjectProperty,
		5 => TagMapping::NameProperty,
		6 => TagMapping::DelegateProperty,
		7 => TagMapping::DoubleProperty,
		8 => {
            let inner_type = Box::new(read_usmap_prop(reader, name_map)?);
            TagMapping::ArrayProperty { inner_type }
        },
		9 => {
            let name = read_usmap_name(reader, name_map)?;
            TagMapping::StructProperty { struct_type: name }
        },
		10 => TagMapping::StrProperty,
		11 => TagMapping::TextProperty,
		12 => TagMapping::InterfaceProperty,
		13 => TagMapping::MulticastDelegateProperty,
		14 => TagMapping::WeakObjectProperty,
		15 => TagMapping::LazyObjectProperty,
		16 => TagMapping::AssetObjectProperty,
		17 => TagMapping::SoftObjectProperty,
		18 => TagMapping::UInt64Property,
		19 => TagMapping::UInt32Property,
		20 => TagMapping::UInt16Property,
		21 => TagMapping::Int64Property,
		22 => TagMapping::Int16Property,
		23 => TagMapping::Int8Property,
		24 => {
            let inner_type = Box::new(read_usmap_prop(reader, name_map)?);
            let value_type = Box::new(read_usmap_prop(reader, name_map)?);
            TagMapping::MapProperty { inner_type, value_type }
        },
		25 => {
            let inner_type = Box::new(read_usmap_prop(reader, name_map)?);
            TagMapping::SetProperty { inner_type }
        },
		26 => {
            let _inner_type = Box::new(read_usmap_prop(reader, name_map)?);
            let enum_name = Some(read_usmap_name(reader, name_map)?);
            TagMapping::EnumProperty { enum_name }
        },
        27 => TagMapping::FieldPathProperty,
        _ => return Err(ParserError::new(format!("Unknown Property Type"))),
    })
}

pub(super) fn read_usmap(data: Vec<u8>) -> ParserResult<(Vec<ClassMapping>, Vec<EnumMapping>)> {
    let header = {
        let mut reader = Cursor::new(data.as_slice());
        UsmapHeader::new(&mut reader)?
    };
    
    let usmap_data = get_usmap_data(data, &header)?;
    let mut reader = Cursor::new(usmap_data.as_slice());


    let mut name_list = Vec::new();
    let name_list_length = reader.read_u32::<LittleEndian>()?;
    for _i in 0..name_list_length {
        name_list.push(read_usmap_string(&mut reader)?);
    }

    let mut enum_mappings = Vec::new();
    let enum_list_length = reader.read_u32::<LittleEndian>()?;
    for _i in 0..enum_list_length {
        let enum_name = read_usmap_name(&mut reader, &name_list)?;
        let enum_values_length = reader.read_u8()?;
        let mut enum_values = Vec::new();
        for _j in 0..enum_values_length {
            enum_values.push(read_usmap_name(&mut reader, &name_list)?);
        }

        let enum_mapping = EnumMapping {
            name: enum_name,
            enum_type: "".to_owned(),
            values: enum_values,
        };

        enum_mappings.push(enum_mapping);
    }

    let mut class_mappings = Vec::new();
    let class_mapping_length = reader.read_u32::<LittleEndian>()?;
    for _i in 0..class_mapping_length {
        let class_name = read_usmap_name(&mut reader, &name_list)?;
        let super_name = read_usmap_name(&mut reader, &name_list)?;
        let prop_count = reader.read_u16::<LittleEndian>()?;
        let contained_prop_count = reader.read_u16::<LittleEndian>()?;
        let mut props = Vec::new();
        
        for _j in 0..contained_prop_count {
            let prop_idx = reader.read_u16::<LittleEndian>()?;
            let _array_size = reader.read_u8()?;
            let prop_name = read_usmap_name(&mut reader, &name_list)?;
            let prop_tag = read_usmap_prop(&mut reader, &name_list)?;

            let prop = PropertyMapping {
                index: prop_idx as u32,
                name: prop_name,
                mapping_type: prop_tag,
            };

            props.push(prop);
        }

        let mapping = ClassMapping {
            name: class_name,
            super_type: match super_name.as_ref() {
                "" => None,
                _ => Some(super_name)
            },
            properties: props,
            property_count: prop_count as u32,
        };

        class_mappings.push(mapping);
    }


    Ok((class_mappings, enum_mappings))
}