use std::cell::RefCell;
use std::rc::Rc;
use serde::Serialize;
use serde::ser::{Serializer, SerializeStruct, SerializeSeq, SerializeMap};

pub struct GLTFContainer {
    pub buffer: Vec<u8>,
    pub data: GLTFItem,
}

pub trait Indexable {
    fn get_index(&self) -> u32;
    fn set_index(&mut self, index: u32);
}

macro_rules! indexable {
    ($class_name:path) => {
        impl Indexable for $class_name {
            fn get_index(&self) -> u32 {
                self.index
            }

            fn set_index(&mut self, index: u32) {
                self.index = index;
            }
        }
    }
}

#[derive(Debug)]
pub struct RefItem<T> {
    data: Rc<RefCell<T>>,
}

impl<T> RefItem<T> {
    fn new(item: Rc<RefCell<T>>) -> Self {
        Self {
            data: item,
        }
    }
}

impl<T> Serialize for RefItem<T> where T: Indexable + Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let item = self.data.borrow();
        serializer.serialize_u32(item.get_index())
    }
}

fn reindex_indexable<T>(items: &Vec<RefOwn<T>>) where T: Indexable {
    let mut index = 0;
    for item in items {
        let mut node = item.data.borrow_mut();
        node.set_index(index);
        index += 1;
    }
}

// Exists only so I can serialize an Rc<RefCell<T>>
#[derive(Debug)]
struct RefOwn<T> {
    data: Rc<RefCell<T>>,
}

impl<T> RefOwn<T> {
    fn new(item: Rc<RefCell<T>>) -> Self {
        Self {
            data: item,
        }
    }
}

impl<T> Serialize for RefOwn<T> where T: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let item = self.data.borrow();
        item.serialize(serializer)
    }
}

#[derive(Debug)]
pub struct GLTFItem {
    asset: GLTFAsset,
    scene: u32,
    scenes: Vec<GLTFScene>,
    nodes: Vec<RefOwn<GLTFNode>>,
    buffers: Vec<GLTFBuffer>,
    buffer_views: Vec<RefOwn<GLTFBufferView>>,
    accessors: Vec<RefOwn<GLTFAccessor>>,
    meshes: Vec<RefOwn<GLTFMesh>>,
    images: Vec<RefOwn<GLTFImage>>,
    samplers: Vec<RefOwn<GLTFSampler>>,
    textures: Vec<RefOwn<GLTFTexture>>,
    materials: Vec<RefOwn<GLTFMaterial>>,
    skins: Vec<RefOwn<GLTFSkin>>,
    animations: Vec<RefOwn<GLTFAnimation>>,
}

impl GLTFItem {
    pub fn new() -> Self {
        GLTFItem {
            asset: GLTFAsset {
                generator: "JohnWickParse",
                version: "2.0",
            },
            scene: 0,
            scenes: vec![GLTFScene {
                name: "MainScene",
                nodes: vec![0],
            }],
            nodes: Vec::new(),
            buffers: Vec::new(),
            buffer_views: Vec::new(),
            accessors: Vec::new(),
            meshes: Vec::new(),
            images: Vec::new(),
            samplers: Vec::new(),
            textures: Vec::new(),
            materials: Vec::new(),
            skins: Vec::new(),
            animations: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: GLTFNode) -> Rc<RefCell<GLTFNode>> {
        let counted = Rc::new(RefCell::new(node));
        self.nodes.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_buffer(&mut self, buffer: GLTFBuffer) {
        self.buffers.push(buffer);
    }

    pub fn add_buffer_view(&mut self, buffer_view: GLTFBufferView) -> Rc<RefCell<GLTFBufferView>> {
        let counted = Rc::new(RefCell::new(buffer_view));
        self.buffer_views.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_accessor(&mut self, accessor: GLTFAccessor) -> Rc<RefCell<GLTFAccessor>> {
        let counted = Rc::new(RefCell::new(accessor));
        self.accessors.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_mesh(&mut self, mesh: GLTFMesh) -> Rc<RefCell<GLTFMesh>> {
        let counted = Rc::new(RefCell::new(mesh));
        self.meshes.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_image(&mut self, image: GLTFImage) -> Rc<RefCell<GLTFImage>> {
        let counted = Rc::new(RefCell::new(image));
        self.images.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_sampler(&mut self, sampler: GLTFSampler) -> Rc<RefCell<GLTFSampler>> {
        let counted = Rc::new(RefCell::new(sampler));
        self.samplers.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_texture(&mut self, texture: GLTFTexture) -> Rc<RefCell<GLTFTexture>> {
        let counted = Rc::new(RefCell::new(texture));
        self.textures.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_material(&mut self, material: GLTFMaterial) -> Rc<RefCell<GLTFMaterial>> {
        let counted = Rc::new(RefCell::new(material));
        self.materials.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_skin(&mut self, skin: GLTFSkin) -> Rc<RefCell<GLTFSkin>> {
        let counted = Rc::new(RefCell::new(skin));
        self.skins.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_animation(&mut self, animation: GLTFAnimation) -> Rc<RefCell<GLTFAnimation>> {
        let counted = Rc::new(RefCell::new(animation));
        self.animations.push(RefOwn::new(counted.clone()));
        counted
    }
}

impl Serialize for GLTFItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        reindex_indexable(&self.nodes);
        reindex_indexable(&self.buffer_views);
        reindex_indexable(&self.accessors);
        reindex_indexable(&self.meshes);
        reindex_indexable(&self.images);
        reindex_indexable(&self.samplers);
        reindex_indexable(&self.materials);
        reindex_indexable(&self.textures);
        reindex_indexable(&self.animations);

        let mut state = serializer.serialize_struct("GLTFItem", 14)?;
        state.serialize_field("asset", &self.asset)?;
        state.serialize_field("scene", &self.scene)?;
        state.serialize_field("scenes", &self.scenes)?;
        state.serialize_field("nodes", &self.nodes)?;
        state.serialize_field("buffers", &self.buffers)?;
        state.serialize_field("bufferViews", &self.buffer_views)?;
        state.serialize_field("accessors", &self.accessors)?;
        state.serialize_field("meshes", &self.meshes)?;
        state.serialize_field("images", &self.images)?;
        state.serialize_field("samplers", &self.samplers)?;
        state.serialize_field("materials", &self.materials)?;
        state.serialize_field("textures", &self.textures)?;
        state.serialize_field("skins", &self.skins)?;
        state.serialize_field("animations", &self.animations)?;

        state.end()
    }
}

#[derive(Debug, Serialize)]
struct GLTFAsset {
    generator: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct GLTFScene {
    name: &'static str,
    nodes: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct GLTFNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<RefItem<GLTFNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mesh: Option<RefItem<GLTFMesh>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    translation: Option<(f32, f32, f32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation: Option<(f32, f32, f32, f32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skin: Option<RefItem<GLTFSkin>>,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFNode {
    pub fn new() -> Self {
        Self {
            name: None,
            children: Vec::new(),
            mesh: None,
            index: 0,
            translation: None,
            rotation: None,
            skin: None,
        }
    }

    pub fn add_child(&mut self, child_node: Rc<RefCell<GLTFNode>>) {
        self.children.push(RefItem::new(child_node));
    }

    pub fn set_mesh(mut self, mesh: Rc<RefCell<GLTFMesh>>) -> Self {
        self.mesh = Some(RefItem::new(mesh));
        self
    }

    pub fn set_position(mut self, translation: (f32, f32, f32), rotation: (f32, f32, f32, f32)) -> Self {
        self.translation = Some(translation);
        self.rotation = Some(rotation);
        self
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn set_skin(&mut self, skin: Rc<RefCell<GLTFSkin>>) {
        self.skin = Some(RefItem::new(skin));
    }

    pub fn get_translation(&self) -> (f32, f32, f32) {
        match self.translation {
            Some(data) => data,
            None => panic!("No translation"),
        }
    }
    
    pub fn get_rotation(&self) -> (f32, f32, f32, f32) {
        match self.rotation {
            Some(data) => data,
            None => panic!("No rotation"),
        }
    }
}

indexable!(GLTFNode);

#[derive(Debug, Serialize)]
pub struct GLTFBuffer {
    #[serde(rename="byteLength")]
    byte_length: u32,
    uri: String,
}

impl GLTFBuffer {
    pub fn new(byte_length: u32, uri: String) -> Self {
        Self {
            byte_length, uri,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFBufferView {
    #[serde(rename="byteLength")]
    byte_length: u32,
    #[serde(rename="byteOffset")]
    byte_offset: u32,
    buffer: u32,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFBufferView {
    pub fn new(byte_offset: u32, byte_length: u32) -> Self {
        GLTFBufferView {
            byte_length, byte_offset,
            buffer: 0,
            index: 0,
        }
    }
}

indexable!(GLTFBufferView);

#[derive(Debug)]
#[allow(dead_code)]
pub enum GLTFComponentType {
    Byte,
    UnsignedByte,
    Short,
    UnsignedShort,
    UnsignedInt,
    Float,
}

impl Serialize for GLTFComponentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let val = match self {
            GLTFComponentType::Byte => 5120,
            GLTFComponentType::UnsignedByte => 5121,
            GLTFComponentType::Short => 5122,
            GLTFComponentType::UnsignedShort => 5123,
            GLTFComponentType::UnsignedInt => 5125,
            GLTFComponentType::Float => 5126,
        };
        serializer.serialize_u32(val)
    }
}

#[derive(Debug)]
pub enum GLTFAccessorValue {
    Vec3Float(f32, f32, f32),
    ScalarFloat(f32),
    None,
}

impl GLTFAccessorValue {
    fn is_none(&self) -> bool {
        match self {
            GLTFAccessorValue::None => true,
            _ => false,
        }
    }
}

impl Serialize for GLTFAccessorValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            GLTFAccessorValue::Vec3Float(x, y, z) => {
                let mut seq = serializer.serialize_seq(Some(3))?;
                seq.serialize_element(x)?;
                seq.serialize_element(y)?;
                seq.serialize_element(z)?;
                seq.end()
            },
            GLTFAccessorValue::ScalarFloat(x) => {
                let mut seq = serializer.serialize_seq(Some(1))?;
                seq.serialize_element(x)?;
                seq.end()
            },
            GLTFAccessorValue::None => {
                serializer.serialize_none()
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFAccessor {
    #[serde(rename="bufferView")]
    buffer_view: RefItem<GLTFBufferView>,
    #[serde(rename="byteOffset")]
    byte_offset: u32,
    #[serde(rename="componentType")]
    component_type: GLTFComponentType,
    count: u32,
    #[serde(rename="type")]
    accessor_type: &'static str,
    #[serde(skip_serializing)]
    index: u32,
    #[serde(skip_serializing_if = "GLTFAccessorValue::is_none")]
    min: GLTFAccessorValue,
    #[serde(skip_serializing_if = "GLTFAccessorValue::is_none")]
    max: GLTFAccessorValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized: Option<bool>,
}

impl GLTFAccessor {
    pub fn new(buffer_view: Rc<RefCell<GLTFBufferView>>, component_type: GLTFComponentType, count: u32, accessor_type: &'static str, min: GLTFAccessorValue, max: GLTFAccessorValue) -> Self {
        Self {
            buffer_view: RefItem::new(buffer_view), 
            component_type, count, accessor_type,
            index: 0,
            byte_offset: 0,
            min, max,
            normalized: None,
        }
    }

    pub fn set_normalized(mut self, normalized: bool) -> Self {
        self.normalized = Some(normalized);
        self
    }
}

indexable!(GLTFAccessor);

#[derive(Debug, Serialize)]
pub struct GLTFMesh {
    primitives: Vec<GLTFPrimitive>,
    #[serde(skip_serializing)]
    index: u32,
}

indexable!(GLTFMesh);

impl GLTFMesh {
    pub fn new(primitives: Vec<GLTFPrimitive>) -> Self {
        Self {
            primitives,
            index: 0,
        }
    }
}

#[derive(Debug)]
pub struct GLTFAttributeMap {
    attributes: Vec<(&'static str, RefItem<GLTFAccessor>)>,
}

impl Serialize for GLTFAttributeMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.attributes.len()))?;
        for attribute in &self.attributes {
            map.serialize_entry(&attribute.0, &attribute.1)?;
        }
        map.end()
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFPrimitive {
    attributes: GLTFAttributeMap,
    indices: RefItem<GLTFAccessor>,
    material: RefItem<GLTFMaterial>,
}

impl GLTFPrimitive {
    pub fn new(indices: Rc<RefCell<GLTFAccessor>>, material: Rc<RefCell<GLTFMaterial>>) -> Self {
        Self {
            attributes: GLTFAttributeMap {
                attributes: Vec::new(),
            },
            indices: RefItem::new(indices),
            material: RefItem::new(material),
        }
    }

    // builder pattern in Rust? yay/nay?
    pub fn add_attribute(mut self, attribute: &'static str, accessor: Rc<RefCell<GLTFAccessor>>) -> Self {
        self.attributes.attributes.push((attribute, RefItem::new(accessor)));

        self
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFImage {
    uri: String,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFImage {
    pub fn new(uri: String) -> Self {
        Self {
            uri,
            index: 0,
        }
    }
}

indexable!(GLTFImage);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GLTFSampler {
    mag_filter: u32,
    min_filter: u32,
    wrap_s: u32,
    wrap_t: u32,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFSampler {
    pub fn new() -> Self {
        Self {
            mag_filter: 9729,
            min_filter: 9729,
            wrap_s: 33071,
            wrap_t: 33071,
            index: 0,
        }
    }
}

indexable!(GLTFSampler);

#[derive(Debug, Serialize)]
pub struct GLTFTexture {
    source: RefItem<GLTFImage>,
    sampler: RefItem<GLTFSampler>,
    #[serde(skip_serializing)]
    index: u32,
}

indexable!(GLTFTexture);

impl GLTFTexture {
    pub fn new(source: Rc<RefCell<GLTFImage>>, sampler: Rc<RefCell<GLTFSampler>>) -> Self {
        Self {
            source: RefItem::new(source),
            sampler: RefItem::new(sampler),
            index: 0,
        }
    }
}

#[derive(Debug, Serialize)]
struct GLTFTextureInfoObject {
    index: RefItem<GLTFTexture>,
}

impl GLTFTextureInfoObject {
    fn new(source: Rc<RefCell<GLTFTexture>>) -> Self {
        Self {
            index: RefItem::new(source),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GLTFPBRMetallicRoughness {
    base_color_texture: GLTFTextureInfoObject,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GLTFMaterial {
    pbr_metallic_roughness: GLTFPBRMetallicRoughness,
    normal_texture: GLTFTextureInfoObject,
    #[serde(skip_serializing)]
    index: u32,
}

indexable!(GLTFMaterial);

impl GLTFMaterial {
    pub fn new(diffuse: Rc<RefCell<GLTFTexture>>, normal: Rc<RefCell<GLTFTexture>>) -> Self {
        Self {
            pbr_metallic_roughness: GLTFPBRMetallicRoughness {
                base_color_texture: GLTFTextureInfoObject::new(diffuse),
            },
            normal_texture: GLTFTextureInfoObject::new(normal),
            index: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFSkin {
    skeleton: RefItem<GLTFNode>,
    joints: Vec<RefItem<GLTFNode>>,
    #[serde(rename="inverseBindMatrices")]
    bind_inverse_matrix: Option<RefItem<GLTFAccessor>>,
    #[serde(skip_serializing)]
    index: u32,
}

indexable!(GLTFSkin);

impl GLTFSkin {
    pub fn new(skeleton: Rc<RefCell<GLTFNode>>, joints: Vec<Rc<RefCell<GLTFNode>>>) -> Self {
        Self {
            skeleton: RefItem::new(skeleton),
            joints: joints.into_iter().map(|v| RefItem::new(v)).collect(),
            index: 0,
            bind_inverse_matrix: None,
        }
    }

    pub fn set_accessor(mut self, accessor: Rc<RefCell<GLTFAccessor>>) -> Self {
        self.bind_inverse_matrix = Some(RefItem::new(accessor));
        self
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum GLTFInterpolation {
    Linear,
    Step,
    CubicSpline,
}

impl Serialize for GLTFInterpolation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(
            match self {
                GLTFInterpolation::Linear => "LINEAR",
                GLTFInterpolation::Step => "STEP",
                GLTFInterpolation::CubicSpline => "CUBICSPLINE",
            }
        )
    }
}

#[derive(Debug)]
pub struct GLTFAnimation {
    channels: Vec<GLTFChannel>,
    samplers: Vec<RefOwn<GLTFAnimationSampler>>,
    index: u32,
}

indexable!(GLTFAnimation);

impl GLTFAnimation {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            samplers: Vec::new(),
            index: 0,
        }
    }

    pub fn add_sampler(&mut self, sampler: GLTFAnimationSampler) -> Rc<RefCell<GLTFAnimationSampler>> {
        let counted = Rc::new(RefCell::new(sampler));
        self.samplers.push(RefOwn::new(counted.clone()));
        counted
    }

    pub fn add_channel(&mut self, channel: GLTFChannel) {
        self.channels.push(channel);
    }
}

impl Serialize for GLTFAnimation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        reindex_indexable(&self.samplers);

        let mut state = serializer.serialize_struct("GLTFAnimation", 2)?;
        state.serialize_field("samplers", &self.samplers)?;
        state.serialize_field("channels", &self.channels)?;

        state.end()
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFChannel {
    sampler: RefItem<GLTFAnimationSampler>,
    target: GLTFAnimationTarget,
}

impl GLTFChannel {
    pub fn new(sampler: Rc<RefCell<GLTFAnimationSampler>>, target: GLTFAnimationTarget) -> Self {
        Self {
            sampler: RefItem::new(sampler),
            target,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GLTFAnimationSampler {
    input: RefItem<GLTFAccessor>,
    interpolation: GLTFInterpolation,
    output: RefItem<GLTFAccessor>,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFAnimationSampler {
    pub fn new(input: Rc<RefCell<GLTFAccessor>>, output: Rc<RefCell<GLTFAccessor>>, interpolation: GLTFInterpolation) -> Self {
        Self {
            input: RefItem::new(input),
            output: RefItem::new(output),
            interpolation,
            index: 0,
        }
    }
}

indexable!(GLTFAnimationSampler);

#[derive(Debug, Serialize)]
pub struct GLTFAnimationTarget {
    path: &'static str,
    extras: String, // going to keep the bone name here
}

impl GLTFAnimationTarget {
    pub fn new(path: &'static str, bone_name: String) -> Self {
        Self {
            path,
            extras: bone_name,
        }
    }
}