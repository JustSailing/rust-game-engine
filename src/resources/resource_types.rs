use std::{cell::RefCell, rc::Rc};

use crate::application::{
    basic::math::{consts::INVALID_ID, vec4::Vec4},
    renderer::vulkan::vulkan_image::VulkanImage,
};
use ash::vk::Sampler;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Texture {
    pub id: usize,
    pub width: u32,
    pub height: u32,
    pub channel_count: u8,
    pub has_transparency: bool,
    pub generation: usize,
    pub internal_data: TextureData,
}

impl Texture {
    pub fn has_transparency(mut self, has_transparency: bool) -> Self {
        self.has_transparency = has_transparency;
        self
    }
    pub fn width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }
    pub fn height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }
    pub fn channel_count(mut self, channel_count: u8) -> Self {
        self.channel_count = channel_count;
        self
    }
    pub fn generation(mut self, generation: usize) -> Self {
        self.generation = generation;
        self
    }
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            width: 0,
            height: 0,
            channel_count: 0,
            has_transparency: false,
            generation: INVALID_ID,
            internal_data: unsafe { std::mem::zeroed() },
        }
    }
}
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TextureData {
    pub image: VulkanImage,
    pub sampler: Sampler,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum TextureUse {
    Unknown = 0x00,
    MapDiffuse = 0x01,
}

#[derive(Debug)]
#[repr(C)]
pub struct TextureMap {
    pub texture: Rc<RefCell<Texture>>,
    pub use_type: TextureUse,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct MaterialConfig {
    pub name: String,
    pub shader_name: String,
    pub auto_release: bool,
    pub diffuse_colour: Vec4,
    pub diffuse_map_name: String,
}

impl Default for MaterialConfig {
    fn default() -> Self {
        Self {
            name: Default::default(),
            auto_release: Default::default(),
            diffuse_colour: Vec4::new_ones(),
            diffuse_map_name: Default::default(),
            shader_name: String::from("Builtin.Material"),
        }
    }
}

impl MaterialConfig {
    pub fn name(mut self, name: &String) -> Self {
        self.name = name.clone();
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct Material {
    pub id: usize,
    pub generation: usize,
    pub internal_id: usize,
    pub shader_id: usize,
    pub name: String,
    pub diffuse_colour: Vec4,
    pub diffuse_map: TextureMap,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            generation: INVALID_ID,
            internal_id: INVALID_ID,
            shader_id: INVALID_ID,
            name: Default::default(),
            diffuse_colour: Vec4::new_ones(),
            diffuse_map: TextureMap {
                texture: Rc::new(RefCell::new(Texture::default())),
                use_type: TextureUse::Unknown,
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct GeometryConfig<T: Clone, U: Clone> {
    pub vertices: Vec<T>,
    pub indices: Vec<U>,
    pub name: String,
    pub material_name: String,
}

#[derive(Debug)]
#[repr(C)]
pub struct Geometry {
    pub id: usize,
    pub internal_id: usize,
    pub generation: usize,
    pub name: String,
    pub material: Rc<RefCell<Material>>,
}

impl Default for Geometry {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            internal_id: INVALID_ID,
            generation: INVALID_ID,
            name: Default::default(),
            material: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum ResourceType {
    Text,
    Binary,
    Image,
    Material,
    StaticMesh,
    Shader,
    Custom,
    Unknown,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub enum ResourceData {
    Unknown,
    ImageResourceData(ImageData),
    MaterialResourceData(MaterialConfig),
    BinaryResourceData(Vec<u8>),
    ShaderResourceData(ShaderConfig),
}

impl Default for ResourceData {
    fn default() -> Self {
        Self::Unknown
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Resource {
    pub loader_id: usize,
    pub name: String,
    pub full_path: String,
    pub data: ResourceData,
}

impl Default for Resource {
    fn default() -> Self {
        Self {
            loader_id: INVALID_ID,
            name: Default::default(),
            full_path: Default::default(),
            data: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ImageData {
    pub channel_count: u8,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub enum ShaderStage {
    Vertex = 0x1,
    Geometry = 0x2,
    Fragment = 0x4,
    Compute = 0x8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum ShaderAttributeType {
    Float32 = 0,
    Float32_2 = 1,
    Float32_3 = 2,
    Float32_4 = 3,
    Matrix4 = 4,
    Int8 = 5,
    Uint8 = 6,
    Int16 = 7,
    Uint16 = 8,
    Int32 = 9,
    Uint32 = 10,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum ShaderUniformType {
    Float32 = 0,
    Float32_2 = 1,
    Float32_3 = 2,
    Float32_4 = 3,
    Int8 = 4,
    Uint8 = 5,
    Int16 = 6,
    Uint16 = 7,
    Int32 = 8,
    Uint32 = 9,
    Matrix4 = 10,
    Sampler = 11,
    Custom = 254,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum ShaderScope {
    Global = 0,
    Instance = 1,
    Local = 2,
    Unknown,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ShaderAttributeConfig {
    pub name: String,
    pub size: usize,
    pub attribute_type: ShaderAttributeType,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct ShaderUniformConfig {
    pub name: String,
    pub size: usize,
    pub location: usize,
    pub uniform_type: ShaderUniformType,
    pub scope: ShaderScope,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ShaderConfig {
    pub name: String,
    pub use_instances: bool,
    pub use_locals: bool,
    pub attributes: Vec<ShaderAttributeConfig>,
    pub uniforms: Vec<ShaderUniformConfig>,
    pub renderpass_name: String,
    pub stages: Vec<ShaderStage>,
    //pub stage_names: Vec<String>,
    pub stage_filenames: Vec<String>,
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self {
            name: Default::default(),
            use_instances: Default::default(),
            use_locals: Default::default(),
            attributes: Default::default(),
            uniforms: Default::default(),
            renderpass_name: Default::default(),
            stages: Default::default(),
            //stage_names: Default::default(),
            stage_filenames: Default::default(),
        }
    }
}
