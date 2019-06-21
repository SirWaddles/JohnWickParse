use byteorder::{LittleEndian, ReadBytesExt};
use super::*;

#[derive(Debug, Serialize)]
struct FMaterialParameterInfo {
    name: String,
    association: u8,
    index: i32,
}

impl NewableWithNameMap for FMaterialParameterInfo {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
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
struct FMaterialUniformExpressionTexture {
    texture_index: i32,
    sampler_source: i32,
}

impl NewableWithNameMap for FMaterialUniformExpressionTexture {
    fn new_n(reader: &mut ReaderCursor, _name_map: &NameMap, _import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            texture_index: reader.read_i32::<LittleEndian>()?,
            sampler_source: reader.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionTextureParameter {
    super_expression: FMaterialUniformExpressionTexture,
    parameter_info: FMaterialParameterInfo,
}

impl NewableWithNameMap for FMaterialUniformExpressionTextureParameter {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            parameter_info: FMaterialParameterInfo::new_n(reader, name_map, import_map)?,
            super_expression: FMaterialUniformExpressionTexture::new_n(reader, name_map, import_map)?,
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
struct FMaterialUniformExpressionComponentSwizzle {
    x: FMaterialUniformExpression,
    index_r: u8,
    index_g: u8,
    index_b: u8,
    index_a: u8,
    num_elements: u8,
}

impl NewableWithNameMap for FMaterialUniformExpressionComponentSwizzle {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            x: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            index_r: reader.read_u8()?,
            index_g: reader.read_u8()?,
            index_b: reader.read_u8()?,
            index_a: reader.read_u8()?,
            num_elements: reader.read_u8()?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionClamp {
    input: FMaterialUniformExpression,
    min: FMaterialUniformExpression,
    max: FMaterialUniformExpression,
}

impl NewableWithNameMap for FMaterialUniformExpressionClamp {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            input: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            min: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            max: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionAbs {
    x: FMaterialUniformExpression,
}

impl NewableWithNameMap for FMaterialUniformExpressionAbs {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            x: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionMax {
    a: FMaterialUniformExpression,
    b: FMaterialUniformExpression,
}

impl NewableWithNameMap for FMaterialUniformExpressionMax {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            a: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
            b: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionCeil {
    x: FMaterialUniformExpression,
}

impl NewableWithNameMap for FMaterialUniformExpressionCeil {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            x: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialUniformExpressionSaturate {
    input: FMaterialUniformExpression,
}

impl NewableWithNameMap for FMaterialUniformExpressionSaturate {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            input: FMaterialUniformExpression::new_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
enum ExpressionType {
    ScalarParameter(FMaterialUniformExpressionScalarParameter),
    VectorParameter(FMaterialUniformExpressionVectorParameter),
    Constant(FMaterialUniformExpressionConstant),
    TextureParameter(FMaterialUniformExpressionTextureParameter),
    AppendVector(FMaterialUniformExpressionAppendVector),
    FoldedMath(FMaterialUniformExpressionFoldedMath),
    ComponentSwizzle(FMaterialUniformExpressionComponentSwizzle),
    Clamp(FMaterialUniformExpressionClamp),
    Abs(FMaterialUniformExpressionAbs),
    Max(FMaterialUniformExpressionMax),
    Ceil(FMaterialUniformExpressionCeil),
    Saturate(FMaterialUniformExpressionSaturate),
    Texture(FMaterialUniformExpressionTexture),
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
            "FMaterialUniformExpressionTextureParameter" => {
                ExpressionType::TextureParameter(FMaterialUniformExpressionTextureParameter::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionFoldedMath" => {
                ExpressionType::FoldedMath(FMaterialUniformExpressionFoldedMath::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionConstant" => {
                ExpressionType::Constant(FMaterialUniformExpressionConstant::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionComponentSwizzle" => {
                ExpressionType::ComponentSwizzle(FMaterialUniformExpressionComponentSwizzle::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionClamp" => {
                ExpressionType::Clamp(FMaterialUniformExpressionClamp::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionAbs" => {
                ExpressionType::Abs(FMaterialUniformExpressionAbs::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionMax" => {
                ExpressionType::Max(FMaterialUniformExpressionMax::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionCeil" => {
                ExpressionType::Ceil(FMaterialUniformExpressionCeil::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionSaturate" => {
                ExpressionType::Saturate(FMaterialUniformExpressionSaturate::new_n(reader, name_map, import_map)?)
            },
            "FMaterialUniformExpressionTexture" => {
                ExpressionType::Texture(FMaterialUniformExpressionTexture::new_n(reader, name_map, import_map)?)
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
    uniform_scalar_expressions: Vec<FMaterialUniformExpression>,
    uniform_2d_texture_expressions: Vec<FMaterialUniformExpression>,
    uniform_cube_texture_expressions: Vec<FMaterialUniformExpression>,
    uniform_volume_texture_expresions: Vec<FMaterialUniformExpression>,
    uniform_external_texture_expressions: Vec<FMaterialUniformExpression>,
}

impl NewableWithNameMap for FUniformExpressionSet {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            uniform_vector_expressions: read_tarray_n(reader, name_map, import_map)?,
            uniform_scalar_expressions: read_tarray_n(reader, name_map, import_map)?,
            uniform_2d_texture_expressions: read_tarray_n(reader, name_map, import_map)?,
            uniform_cube_texture_expressions: read_tarray_n(reader, name_map, import_map)?,
            uniform_volume_texture_expresions: read_tarray_n(reader, name_map, import_map)?,
            uniform_external_texture_expressions: read_tarray_n(reader, name_map, import_map)?,
        })
    }
}

#[derive(Debug, Serialize)]
struct FMaterialCompilationOutput {
    uniform_expression_set: FUniformExpressionSet,
    used_scene_textures: u64,
    requires_scene_colour_copy: bool,
    needs_scene_textures: bool,
    uses_eye_adaptation: bool,
    modifies_mesh_position: bool,
    uses_world_position_offset: bool,
    needs_gbuffer: bool,
    uses_global_distance_field: bool,
    uses_pixel_depth_offset: bool,
    uses_scene_depth_lookup: bool,
    uses_velocity_scene_texture: bool,
    uses_distance_cull_fade: bool,
}

impl NewableWithNameMap for FMaterialCompilationOutput {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        Ok(Self {
            uniform_expression_set: FUniformExpressionSet::new_n(reader, name_map, import_map)?,
            used_scene_textures: reader.read_u64::<LittleEndian>()?,
            requires_scene_colour_copy: reader.read_u32::<LittleEndian>()? != 0,
            needs_scene_textures: reader.read_u32::<LittleEndian>()? != 0,
            uses_eye_adaptation: reader.read_u32::<LittleEndian>()? != 0,
            modifies_mesh_position: reader.read_u32::<LittleEndian>()? != 0,
            uses_world_position_offset: reader.read_u32::<LittleEndian>()? != 0,
            needs_gbuffer: reader.read_u32::<LittleEndian>()? != 0,
            uses_global_distance_field: reader.read_u32::<LittleEndian>()? != 0,
            uses_pixel_depth_offset: reader.read_u32::<LittleEndian>()? != 0,
            uses_scene_depth_lookup: reader.read_u32::<LittleEndian>()? != 0,
            uses_velocity_scene_texture: reader.read_u32::<LittleEndian>()? != 0,
            uses_distance_cull_fade: reader.read_u32::<LittleEndian>()? != 0,
        })
    }
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct FMaterialShaderMap {
    map_id: FMaterialShaderMapId,
    friendly_name: String,
    compilation_output: FMaterialCompilationOutput,
}

impl NewableWithNameMap for FMaterialShaderMap {
    fn new_n(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let _map_id = FMaterialShaderMapId::new(reader)?;

        // Looks like some stuff was added here?
        // It kinda looks like a guid, but there's an extra 4 bytes. So I'm not really sure.
        let _id_guid = FGuid::new(reader)?;
        let _another_val = reader.read_u32::<LittleEndian>()?;
        let _platform = reader.read_i32::<LittleEndian>()?;
        let _friendly_name = read_string(reader)?;
        let _compilation_output = FMaterialCompilationOutput::new_n(reader, name_map, import_map)?;
        println!("pos: {}", reader.position());
        // No idea what these do
        let _word1 = reader.read_u32::<LittleEndian>()?;
        let _word2 = reader.read_u32::<LittleEndian>()?;
        let _word3 = reader.read_u32::<LittleEndian>()?;
        let _word4 = reader.read_u16::<LittleEndian>()?;
        let debug_description = read_string(reader)?;
        println!("debug: {}", debug_description);

        return Err(ParserError::new(format!("Not implemented: now at position {}", reader.position())));

        /*Ok(Self {
            map_id, friendly_name, compilation_output,
        })*/
    }
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
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

#[allow(dead_code)]
impl UMaterialInstanceConstant {
    pub(super) fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "MaterialInstanceConstant")?;
        let num_resources = reader.read_i32::<LittleEndian>()?;

        let resource_name_map: Vec<FNameEntrySerialized> = read_tarray(reader)?;
        let _locs: Vec<FMaterialResourceLocOnDisk> = read_tarray(reader)?;
        let _num_bytes = reader.read_u32::<LittleEndian>()?;

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

impl PackageExport for UMaterialInstanceConstant {
    fn get_export_type(&self) -> &str {
        "MaterialInstanceConstant"
    }
}