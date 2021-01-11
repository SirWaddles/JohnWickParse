use std::io::Read;
use std::fs::File;
use std::path::Path;
use serde::Deserialize;
use crate::assets::{ParserResult, ParserError, ParserType};

mod smrt;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum TagMapping {
    BoolProperty,
    ByteProperty,
    EnumProperty { enum_name: Option<String> },
    TextProperty,
    StrProperty,
    NameProperty,
    ArrayProperty { 
        #[serde(default)]
        inner_type: Box<TagMapping> 
    },
    MapProperty { inner_type: Box<TagMapping>, value_type: Box<TagMapping> },
    ObjectProperty,
    StructProperty { struct_type: String },
    DebugProperty,
    SetProperty { 
        #[serde(default)]
        inner_type: Box<TagMapping> 
    },
    Int8Property,
    Int16Property,
    IntProperty,
    Int64Property,
    UInt16Property,
    UInt32Property,
    UInt64Property,
    FloatProperty,
    DoubleProperty,
    WeakObjectProperty,
    LazyObjectProperty,
    SoftObjectProperty,
    DelegateProperty,
    MulticastDelegateProperty,
    InterfaceProperty,
    FieldPathProperty,
    AssetObjectProperty,
}

impl Default for TagMapping {
    fn default() -> TagMapping {
        TagMapping::DebugProperty
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PropertyMapping {
    index: u32,
    name: String,
    mapping_type: TagMapping,
}

impl PropertyMapping {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> &TagMapping {
        &self.mapping_type
    }
}

#[derive(Debug, Deserialize)]
struct ClassMapping {
    name: String,
    super_type: Option<String>,
    properties: Vec<PropertyMapping>,
    property_count: u32,
}

impl ClassMapping {
    fn get_properties_offset(&self, offset: u32) -> Vec<PropertyMapping> {
        self.properties.iter().map(|v| {
            let mut offset_prop = v.clone();
            offset_prop.index += offset;
            offset_prop
        }).collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct EnumMapping {
    name: String,
    #[serde(rename="type")]
    enum_type: String,
    values: Vec<String>,
}

#[derive(Debug)]
pub struct MappingStore {
    class_mappings: Vec<ClassMapping>,
    enum_mappings: Vec<EnumMapping>,
}

fn get_files(path: &str, file_ext: &str) -> ParserResult<Vec<String>> {
    let path_dir = Path::new(path);
    let mapping_files: Vec<String> = path_dir.read_dir()?.filter(|v| {
        match v {
            Ok(path) => match path.path().extension() {
                Some(ext) => ext == file_ext,
                None => false,
            },
            _ => false,
        }
    }).map(|v| path_dir.to_str().unwrap().to_owned() + v.unwrap().file_name().to_str().unwrap()).collect();

    Ok(mapping_files)
}

impl MappingStore {
    pub fn build_mappings() -> ParserResult<Self> {
        let class_files = get_files("mappings/classes/", "json")?;

        let mut class_mappings = Vec::new();
        for file in class_files {
            let mut file = File::open(file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            let mut store_mappings: Vec<ClassMapping> = match serde_json::from_str(&contents) {
                Ok(m) => m,
                Err(e) => {
                    println!("JSON Error: {}", e);
                    continue;
                },
            };

            class_mappings.append(&mut store_mappings);
        }

        let enum_files = get_files("mappings/enums/", "json")?;
        let mut enum_mappings = Vec::new();
        for file in enum_files {
            let mut file = File::open(file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            let mut store_mappings: Vec<EnumMapping> = match serde_json::from_str(&contents) {
                Ok(m) => m,
                Err(e) => {
                    println!("JSON Error: {}", e);
                    continue;
                },
            };

            enum_mappings.append(&mut store_mappings);
        }

        let usmap_files = get_files("mappings/", "usmap")?;
        for file in usmap_files {
            let (mut n_class_mappings, mut n_enum_mappings) = smrt::read_usmap(std::fs::read(file)?)?;
            class_mappings.append(&mut n_class_mappings);
            enum_mappings.append(&mut n_enum_mappings);
        }

        Ok(Self {
            class_mappings,
            enum_mappings,
        })
    }

    pub fn empty() -> Self {
        Self {
            class_mappings: Vec::new(),
            enum_mappings: Vec::new(),
        }
    }

    fn find_class_mapping(&self, class_name: &str) -> ParserResult<&ClassMapping> {
        match self.class_mappings.iter().find(|v| v.name == class_name) {
            Some(map) => Ok(map),
            None => Err(ParserError::typed(format!("Class not found: {}", class_name), ParserType::ClassMappingMissing)),
        }
    }

    pub fn get_mappings(&self, class_name: &str, indices: Vec<u32>) -> ParserResult<Vec<PropertyMapping>> {
        let class_mapping = self.find_class_mapping(class_name)?;

        let mut properties = class_mapping.get_properties_offset(0);
        let mut total_offset = class_mapping.property_count;
        let mut target_class = class_mapping.super_type.clone();
        loop {
            match target_class {
                Some(ref target) => {
                    match self.find_class_mapping(target) {
                        Ok(mapping) => {
                            properties.append(&mut mapping.get_properties_offset(total_offset));
                            total_offset += mapping.property_count;
                            target_class = mapping.super_type.clone();
                        },
                        Err(_e) => {
                            break;
                        },
                    };
                },
                None => break,
            };
        }

        let mut mappings = Vec::new();
        for index in &indices {
            let mapping = match properties.iter().find(|v| &v.index == index) {
                Some(map) => map,
                None => return Err(ParserError::typed(format!("Index not found: {}-{} of {:#?}", class_name, index, indices), ParserType::PropertyIndexMissing)),
            };
            mappings.push(mapping.clone());
        }

        Ok(mappings)
    }

    pub fn get_enum_mapping(&self, enum_name: &Option<String>, idx: usize) -> Option<&str> {
        match enum_name {
            Some(name) => match self.enum_mappings.iter().find(|v| &v.name == name) {
                Some(mapping) => match mapping.values.get(idx) {
                    Some(val) => Some(val),
                    None => None,
                },
                None => None,
            },
            None => None,
        }
    }
}