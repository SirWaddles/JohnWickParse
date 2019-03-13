#[derive(Debug, Serialize)]
pub struct GLTFItem<'a> {
    asset: GLTFAsset,
    scene: u32,
    scenes: Vec<GLTFScene>,
    nodes: Vec<GLTFNode<'a>>,
    buffers: Vec<GLTFBuffer>,
    #[serde(rename="bufferViews")]
    buffer_views: Vec<GLTFBufferView>,
}

impl<'a> GLTFItem<'a> {
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
        }
    }

    pub fn new_node<'b>(&'b mut self) -> &'a mut GLTFNode {
        let index = self.nodes.len();
        let node = GLTFNode {
            name: None,
            children: Vec::new(),
        };

        self.nodes.push(node);
        self.nodes.get_mut(index).unwrap()
    }

    pub fn add_buffer<'b>(&'b mut self, buffer: GLTFBuffer) {
        self.buffers.push(buffer);
    }

    pub fn add_buffer_view(&'a mut self, buffer_view: GLTFBufferView) -> &'a GLTFBufferView {
        let index = self.buffer_views.len();
        self.buffer_views.push(buffer_view);
        self.buffer_views.get(index).unwrap()
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
pub struct GLTFNode<'a> {
    name: Option<String>,
    children: Vec<&'a GLTFNode<'a>>,
}

impl<'a> GLTFNode<'a> {
    pub fn add_child(&'a mut self, child_node: &'a GLTFNode) {
        self.children.push(child_node);
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
}

impl GLTFBufferView {
    pub fn new(byte_offset: u32, byte_length: u32) -> Self {
        GLTFBufferView {
            byte_length, byte_offset,
            buffer: 0,
        }
    }
}