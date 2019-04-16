use byteorder::{LittleEndian, ReadBytesExt};
use super::*;

#[derive(Debug, Serialize)]
struct FMaterialParameterInfo {
    name: String,
    association: u8,
    index: i32,
}

impl NewableWithNameMap for FMaterialParameterInfo {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            name: read_fname(reader, name_map)?,
            association: reader.read_u8()?,
            index: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialShaderMapId {
    quality_level: u32,
    feature_level: u32,
}

impl Newable for FMaterialShaderMapId {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            quality_level: reader.read_u32::<LittleEndian>()?,
            feature_level: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionConstant {
    value: FLinearColor,
    value_type: u8,
}

impl NewableWithNameMap for FMaterialUniformExpressionConstant {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            value: FLinearColor::new_n(reader, name_map, import_map)?,
            value_type: reader.read_u8()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionScalarParameter {
    parameter_info: FMaterialParameterInfo,
    default_value: f32,
}

impl NewableWithNameMap for FMaterialUniformExpressionScalarParameter {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            parameter_info: FMaterialParameterInfo::new_n(reader, name_map, import_map)?,
            default_value: reader.read_f32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionVectorParameter {
    parameter_info: FMaterialParameterInfo,
    default_value: FLinearColor,
}

impl NewableWithNameMap for FMaterialUniformExpressionVectorParameter {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            parameter_info: FMaterialParameterInfo::new_n(reader, name_map, import_map)?,
            default_value: FLinearColor::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionAppendVector {
    a: FMaterialUniformExpression,
    b: FMaterialUniformExpression,
    num_components: u32,
}

impl NewableWithNameMap for FMaterialUniformExpressionAppendVector {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            a: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            b: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            num_components: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionFoldedMath {
    a: FMaterialUniformExpression,
    b: FMaterialUniformExpression,
    value_type: u32,
    operation: u8,
}

impl NewableWithNameMap for FMaterialUniformExpressionFoldedMath {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            a: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            b: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            value_type: reader.read_u32::<LittleEndian>()?,
            operation: reader.read_u8()?,
        })
    }
}

#[derive(Debug, Serialize)]
enum ExpressionType {
    ScalarParameter(FMaterialUniformExpressionScalarParameter),
    VectorParameter(FMaterialUniformExpressionVectorParameter),
    ExpressionConstant(FMaterialUniformExpressionConstant),
    AppendVector(FMaterialUniformExpressionAppendVector),
    FoldedMath(FMaterialUniformExpressionFoldedMath),
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpression {
    expression_type: String,
    expression: Box<ExpressionType>,
}

impl NewableWithNameMap for FMaterialUniformExpression {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let type_name = read_fname(reader, name_map)?;
        let expression = Box::new(match type_name.as_ref() {
            "FMaterialUniformExpressionAppendVector" => {
                ExpressionType::AppendVector(FMaterialUniformExpressionAppendVector::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionScalarParameter" => {
                ExpressionType::ScalarParameter(FMaterialUniformExpressionScalarParameter::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionVectorParameter" => {
                ExpressionType::VectorParameter(FMaterialUniformExpressionVectorParameter::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionFoldedMath" => {
                ExpressionType::FoldedMath(FMaterialUniformExpressionFoldedMath::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionConstant" => {
                ExpressionType::ExpressionConstant(FMaterialUniformExpressionConstant::new_n(reader, name_map, import_map)?)
            },
            _ => {
                return Err(ParserError::new(format!("Incompatible Type: {}", type_name)));
            }
        });
        Ok(Self {
            expression_type: type_name,
            expression,
        })
    }
}

#[derive(Debug, Serialize)]
struct FUniformExpressionSet {
    uniform_vector_expressions: Vec<FMaterialUniformExpression>,
}

impl NewableWithNameMap for FUniformExpressionSet {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            uniform_vector_expressions: read_tarray_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialCompilationOutput {
    uniform_expression_set: FUniformExpressionSet,
}

impl NewableWithNameMap for FMaterialCompilationOutput {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            uniform_expression_set: FUniformExpressionSet::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialShaderMap {
    map_id: FMaterialShaderMapId,
    compilation_output: FMaterialCompilationOutput,
}

impl NewableWithNameMap for FMaterialShaderMap {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let map_id = FMaterialShaderMapId::new(reader)?;
        println!("{:#?} {}", map_id, reader.position());

        // Looks like some stuff was added here?
        // It kinda looks like a guid, but there's an extra 4 bytes. So I'm not really sure.
        let id_guid = FGuid::new(reader)?;
        let another_val = reader.read_u32::<LittleEndian>()?;
        let platform = reader.read_i32::<LittleEndian>()?;
        println!("platform: {} {}", platform, id_guid);
        let friendly_name = read_string(reader)?;
        println!("friendly name: {}", friendly_name);
        let compilation_output = FMaterialCompilationOutput::new_n(reader, name_map, import_map)?;
        Err(ParserError::new(format!("Not implemented")))
    }
}

#[derive(Debug, Serialize)]
struct FMaterialResourceLocOnDisk {
    offset: u32,
    feature_level: u8,
    quality_level: u8,
}

impl Newable for FMaterialResourceLocOnDisk {
    fn new(reader: &mut ReaderCursor) -> ParserResult<Self> {
        Ok(Self {
            offset: reader.read_u32::<LittleEndian>()?,
            feature_level: reader.read_u8()?,
            quality_level: reader.read_u8()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct UMaterialInstanceConstant {
    super_object: UObject,
}

impl UMaterialInstanceConstant {
    pub(super) fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "MaterialInstanceConstant")?;
        let num_resources = reader.read_i32::<LittleEndian>()?;

        let resource_name_map: Vec<FNameEntrySerialized> = read_tarray(reader)?;
        let locs: Vec<FMaterialResourceLocOnDisk> = read_tarray(reader)?;
        let num_bytes = reader.read_u32::<LittleEndian>()?;

        let mut shader_maps = Vec::new();
        for _i in 0..num_resources {
            let cooked = reader.read_u32::<LittleEndian>()? != 0;
            if !cooked { continue; }
            let valid = reader.read_u32::<LittleEndian>()? != 0;
            if !valid { continue; }
            shader_maps.push(FMaterialShaderMap::new_n(reader, &resource_name_map, import_map)?);
        }

        println!("{:#?}", shader_maps);

        Ok(Self {
            super_object,
        })
    }
}