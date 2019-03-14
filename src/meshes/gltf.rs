use std::cell::RefCell;
use std::rc::Rc;
use serde::ser::{Serialize, Serializer, SerializeStruct};

pub trait Indexable {
    fn get_index(&self) -> u32;
    fn set_index(&mut self, index: u32);
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
        }
    }

    pub fn new_node(&mut self) -> Rc<RefCell<GLTFNode>> {
        let node = Rc::new(RefCell::new(GLTFNode {
            name: None,
            children: Vec::new(),
            index: 0,
        }));

        self.nodes.push(RefOwn::new(node.clone()));
        node
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
}

impl Serialize for GLTFItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        reindex_indexable(&self.nodes);
        reindex_indexable(&self.buffer_views);
        reindex_indexable(&self.accessors);

        let mut state = serializer.serialize_struct("GLTFItem", 7)?;
        state.serialize_field("asset", &self.asset)?;
        state.serialize_field("scene", &self.scene)?;
        state.serialize_field("scenes", &self.scenes)?;
        state.serialize_field("nodes", &self.nodes)?;
        state.serialize_field("buffers", &self.buffers)?;
        state.serialize_field("bufferViews", &self.buffer_views)?;
        state.serialize_field("accessors", &self.accessors)?;

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
    name: Option<String>,
    children: Vec<RefItem<GLTFNode>>,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFNode {
    pub fn add_child(mut self, child_node: Rc<RefCell<GLTFNode>>) {
        self.children.push(RefItem::new(child_node));
    }
}

impl Indexable for GLTFNode {
    fn get_index(&self) -> u32 {
        self.index
    }

    fn set_index(&mut self, index: u32) {
        self.index = index;
    }
}

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

impl Indexable for GLTFBufferView {
    fn get_index(&self) -> u32 {
        self.index
    }

    fn set_index(&mut self, index: u32) {
        self.index = index;
    }
}

#[derive(Debug)]
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

#[derive(Debug, Serialize)]
pub struct GLTFAccessor {
    #[serde(rename="bufferView")]
    buffer_view: RefItem<GLTFBufferView>,
    #[serde(rename="byteOffset")]
    byte_offset: u32,
    component_type: GLTFComponentType,
    count: u32,
    #[serde(rename="type")]
    accessor_type: &'static str,
    #[serde(skip_serializing)]
    index: u32,
}

impl GLTFAccessor {
    pub fn new(buffer_view: Rc<RefCell<GLTFBufferView>>, component_type: GLTFComponentType, count: u32, accessor_type: &'static str) -> Self {
        Self {
            buffer_view: RefItem::new(buffer_view), 
            component_type, count, accessor_type,
            index: 0,
            byte_offset: 0,
        }
    }
}

impl Indexable for GLTFAccessor {
    fn get_index(&self) -> u32 {
        self.index
    }

    fn set_index(&mut self, index: u32) {
        self.index = index;
    }
}