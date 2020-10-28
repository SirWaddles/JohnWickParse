use std::io::Read;
use std::fs::File;
use std::path::Path;
use serde::Deserialize;
use serde_json::Result as JSONResult;
use crate::assets::{ParserResult, ParserError, ParserType};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum TagMapping {
    BoolProperty,
    ByteProperty,
    EnumProperty { enum_type: Option<String> },
    IntProperty,
    FloatProperty,
    TextProperty,
    StrProperty,
    NameProperty,
    ArrayProperty { sub_type: Box<TagMapping> },
    MapProperty { key_type: Box<TagMapping>, value_type: Box<TagMapping> },
    ObjectProperty,
    SoftObjectProperty,
    StructProperty { struct_type: String },
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
    properties: Vec<PropertyMapping>,
}

#[derive(Debug, Deserialize)]
pub struct EnumMapping {
    name: String,
    values: Vec<String>,
}

#[derive(Debug)]
pub struct MappingStore {
    class_mappings: Vec<ClassMapping>,
    enum_mappings: Vec<EnumMapping>,
}

fn get_json_files(path: &str) -> ParserResult<Vec<String>> {
    let path_dir = Path::new(path);
    let mapping_files: Vec<String> = path_dir.read_dir()?.filter(|v| {
        match v {
            Ok(path) => match path.path().extension() {
                Some(ext) => ext == "json",
                None => false,
            },
            _ => false,
        }
    }).map(|v| path_dir.to_str().unwrap().to_owned() + v.unwrap().file_name().to_str().unwrap()).collect();

    Ok(mapping_files)
}

impl MappingStore {
    pub fn build_mappings() -> ParserResult<Self> {
        let class_files = get_json_files("mappings/classes/")?;

        let mut class_mappings = Vec::new();
        for file in class_files {
            let mut file = File::open(file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            match serde_json::from_str(&contents) {
                Ok(m) => class_mappings.push(m),
                Err(e) => println!("JSON Error: {}", e),
            };
        }

        let enum_files = get_json_files("mappings/enums/")?;
        let mut enum_mappings = Vec::new();
        for file in enum_files {
            let mut file = File::open(file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            match serde_json::from_str(&contents) {
                Ok(m) => enum_mappings.push(m),
                Err(e) => println!("JSON Error: {}", e),
            };
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

    pub fn get_mappings(&self, class_name: &str, indices: Vec<u32>) -> ParserResult<Vec<PropertyMapping>> {
        let class_mapping = match self.class_mappings.iter().find(|v| v.name == class_name) {
            Some(map) => map,
            None => return Err(ParserError::typed(format!("Class not found: {}", class_name), ParserType::ClassMappingMissing)),
        };

        let mut mappings = Vec::new();
        for index in &indices {
            let mapping = match class_mapping.properties.iter().find(|v| &v.index == index) {
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