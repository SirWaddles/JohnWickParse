use std::io::Read;
use std::fs::File;
use std::path::Path;
use serde::Deserialize;
use serde_json::Result as JSONResult;
use crate::assets::{ParserResult, ParserError};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum TagMapping {
    BoolProperty,
    ByteProperty,
    EnumProperty,
    TextProperty,
    StrProperty,
    ArrayProperty,
    ObjectProperty,
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

#[derive(Debug)]
pub struct MappingStore {
    mappings: Vec<ClassMapping>,
}

impl MappingStore {
    pub fn build_mappings() -> ParserResult<Self> {
        let path_dir = Path::new("mappings/");
        let mapping_files: Vec<String> = path_dir.read_dir()?.filter(|v| {
            match v {
                Ok(path) => match path.path().extension() {
                    Some(ext) => ext == "json",
                    None => false,
                },
                _ => false,
            }
        }).map(|v| path_dir.to_str().unwrap().to_owned() + v.unwrap().file_name().to_str().unwrap()).collect();

        let mut mappings = Vec::new();
        for file in mapping_files {
            let mut file = File::open(file)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            match serde_json::from_str(&contents) {
                Ok(m) => mappings.push(m),
                Err(e) => println!("JSON Error: {}", e),
            };
        }

        Ok(Self {
            mappings,
        })
    }

    pub fn empty() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    pub fn get_mappings(&self, class_name: &str, indices: Vec<u32>) -> ParserResult<Vec<PropertyMapping>> {
        let class_mapping = match self.mappings.iter().find(|v| v.name == class_name) {
            Some(map) => map,
            None => return Err(ParserError::new(format!("Class not found: {}", class_name))),
        };

        let mut mappings = Vec::new();
        for index in indices {
            let mapping = match class_mapping.properties.iter().find(|v| v.index == index) {
                Some(map) => map,
                None => return Err(ParserError::new(format!("Index not found: {}", index))),
            };
            mappings.push(mapping.clone());
        }

        Ok(mappings)
    }
}