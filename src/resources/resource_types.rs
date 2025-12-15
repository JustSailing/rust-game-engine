use crate::basic::math::vec3::{Extents3D, Vec3, Vector3D};
use crate::{
    basic::math::{consts::INVALID_ID, transform::Transform, vec4::Vec4},
    renderer::vulkan::vulkan_image::VulkanImage,
};
use ash::vk::{Filter, Sampler, SamplerAddressMode};
use bitflags::bitflags;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Texture {
    pub id: usize,
    pub texture_type: TextureType,
    pub width: u32,
    pub height: u32,
    pub channel_count: u8,
    pub flags: TextureFlags,
    pub generation: usize,
    pub name: String,
    pub internal_data: TextureData,
}

impl Texture {
    pub fn transparency_flag(mut self, transparency: bool) -> Self {
        if transparency {
            self.flags.insert(TextureFlags::Transparency);
        } else {
            self.flags.remove(TextureFlags::Transparency);
        }
        self
    }
    pub fn texture_type(mut self, texture_type: TextureType) -> Self {
        self.texture_type = texture_type;
        self
    }
    pub fn id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }
    pub fn writeable_flag(mut self, is_writeable: bool) -> Self {
        if is_writeable {
            self.flags.insert(TextureFlags::Writable);
        } else {
            self.flags.remove(TextureFlags::Writable);
        }
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
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            texture_type: TextureType::_2D,
            width: 0,
            height: 0,
            channel_count: 0,
            flags: Default::default(),
            generation: INVALID_ID,
            name: Default::default(),
            internal_data: Default::default(),
        }
    }
}

bitflags! {
    #[repr(C)]
    #[derive(Debug,Default, Clone, Copy, PartialEq, Eq)]
    pub struct TextureFlags: u32 {
        const Transparency = 0x1;
        const Writable = 0x2;
        const Wrapped = 0x4;
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureType {
    #[default]
    _2D,
    Cube,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TextureData {
    pub image: VulkanImage,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum TextureUse {
    #[default]
    Unknown = 0x00,
    DiffuseMap = 0x01,
    SpecularMap = 0x02,
    NormalMap = 0x03,
    CubeMap = 0x4,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum TextureFilter {
    Nearest = 0x0,
    Linear = 0x1,
}

impl Into<Filter> for TextureFilter {
    fn into(self) -> Filter {
        match self {
            TextureFilter::Nearest => Filter::NEAREST,
            TextureFilter::Linear => Filter::LINEAR,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub enum TextureRepeat {
    #[default]
    Repeat = 0x1,
    MirroredRepeat = 0x2,
    ClampToEdge = 0x3,
    ClampToBorder = 0x4,
}

impl Into<SamplerAddressMode> for TextureRepeat {
    fn into(self) -> SamplerAddressMode {
        match self {
            TextureRepeat::Repeat => SamplerAddressMode::REPEAT,
            TextureRepeat::MirroredRepeat => SamplerAddressMode::MIRRORED_REPEAT,
            TextureRepeat::ClampToEdge => SamplerAddressMode::CLAMP_TO_EDGE,
            TextureRepeat::ClampToBorder => SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

pub type TextureHandle = usize;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct TextureMap {
    pub texture_handle: TextureHandle,
    pub texture_name: String,
    pub use_type: TextureUse,
    pub filter_minify: TextureFilter,
    pub filter_magnify: TextureFilter,
    pub repeat_u: TextureRepeat,
    pub repeat_v: TextureRepeat,
    pub repeat_w: TextureRepeat,
    pub internal_data: Sampler,
}

impl Default for TextureMap {
    fn default() -> Self {
        Self {
            texture_handle: INVALID_ID,
            texture_name: Default::default(),
            use_type: TextureUse::Unknown,
            filter_minify: TextureFilter::Linear,
            filter_magnify: TextureFilter::Linear,
            repeat_u: TextureRepeat::Repeat,
            repeat_v: TextureRepeat::Repeat,
            repeat_w: TextureRepeat::Repeat,
            internal_data: Sampler::null(),
        }
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct MaterialConfig {
    pub name: String,
    pub auto_release: bool,
    pub diffuse_colour: Vec4,
    pub shininess: f32,
    pub shader_name: String,
    pub diffuse_map_name: String,
    pub specular_map_name: String,
    pub normal_map_name: String,
}

impl Default for MaterialConfig {
    fn default() -> Self {
        Self {
            name: Default::default(),
            auto_release: Default::default(),
            diffuse_colour: Vec4::new_ones(),
            shininess: Default::default(), // might change this to 32.0
            shader_name: String::from(""),
            diffuse_map_name: Default::default(),
            specular_map_name: Default::default(),
            normal_map_name: Default::default(),
        }
    }
}

impl MaterialConfig {
    pub fn name(mut self, name: &String) -> Self {
        self.name = name.clone();
        self
    }

    pub fn shader_name(mut self, name: &String) -> Self {
        self.shader_name = name.clone();
        self
    }

    pub fn auto_release(mut self, auto_release: bool) -> Self {
        self.auto_release = auto_release;
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
    pub render_frame_number: u64,
    pub name: String,
    pub diffuse_colour: Vec4,
    pub diffuse_map: TextureMap,
    pub specular_map: TextureMap,
    pub normal_map: TextureMap,
    pub shininess: f32,
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
            // for texture repeat and filter
            // it would probably be a good idea to have an Unknown variant
            diffuse_map: TextureMap {
                texture_handle: INVALID_ID,
                texture_name: Default::default(),
                use_type: TextureUse::Unknown,
                filter_minify: TextureFilter::Nearest,
                filter_magnify: TextureFilter::Nearest,
                repeat_u: TextureRepeat::Repeat,
                repeat_v: TextureRepeat::Repeat,
                repeat_w: TextureRepeat::Repeat,
                internal_data: Sampler::null(),
            },
            specular_map: TextureMap {
                texture_handle: INVALID_ID,
                texture_name: Default::default(),
                use_type: TextureUse::Unknown,
                filter_minify: TextureFilter::Nearest,
                filter_magnify: TextureFilter::Nearest,
                repeat_u: TextureRepeat::Repeat,
                repeat_v: TextureRepeat::Repeat,
                repeat_w: TextureRepeat::Repeat,
                internal_data: Sampler::null(),
            },
            normal_map: TextureMap {
                texture_handle: INVALID_ID,
                texture_name: Default::default(),
                use_type: TextureUse::Unknown,
                filter_minify: TextureFilter::Nearest,
                filter_magnify: TextureFilter::Nearest,
                repeat_u: TextureRepeat::Repeat,
                repeat_v: TextureRepeat::Repeat,
                repeat_w: TextureRepeat::Repeat,
                internal_data: Sampler::null(),
            },
            shininess: Default::default(),
            render_frame_number: INVALID_ID as u64,
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct GeometryConfig<T: Clone, U: Clone> {
    pub vertices: Vec<T>,
    pub indices: Vec<U>,
    pub center: Vec3,
    pub min_extents: Vec3,
    pub max_extents: Vec3,
    pub name: String,
    pub material_name: String,
}

impl Default for GeometryConfig<Vector3D, u32> {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            indices: Default::default(),
            center: Default::default(),
            min_extents: Default::default(),
            max_extents: Default::default(),
            name: Default::default(),
            material_name: Default::default(),
        }
    }
}

pub type MaterialHandle = usize;
pub type GeometryHandle = usize;

#[derive(Debug)]
#[repr(C)]
pub struct Geometry {
    pub id: usize,
    pub internal_id: usize,
    pub material_instance_id: usize,
    pub generation: usize,
    pub center: Vec3,
    pub extents: Extents3D,
    pub name: String,
    pub material_handle: MaterialHandle,
    pub material_name: String,
}

impl Default for Geometry {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            internal_id: INVALID_ID,
            generation: INVALID_ID,
            extents: Default::default(),
            center: Vec3::new_zeroes(),
            name: Default::default(),
            material_handle: INVALID_ID,
            material_name: Default::default(),
            material_instance_id: INVALID_ID,
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Mesh {
    pub geometries: Vec<GeometryHandle>,
    pub transform: Rc<RefCell<Transform>>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Skybox {
    pub cube_map: TextureMap,
    pub geometry_handle: GeometryHandle,
    pub instance_id: usize,
    pub render_frame_number: usize,
}

impl Default for Skybox {
    fn default() -> Self {
        Self {
            cube_map: Default::default(),
            geometry_handle: INVALID_ID,
            instance_id: INVALID_ID,
            render_frame_number: INVALID_ID,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum ResourceType {
    #[default]
    Unknown,
    Text,
    Binary,
    Image,
    Material,
    Mesh,
    Shader,
    Custom,
}

#[derive(Default, Clone)]
#[repr(C)]
pub enum ResourceData {
    #[default]
    Unknown,
    ImageResourceData(ImageData),
    MaterialResourceData(MaterialConfig),
    BinaryResourceData(Vec<u8>),
    ShaderResourceData(ShaderConfig),
    MeshResourceData(Vec<GeometryConfig<Vector3D, u32>>),
}

bitflags! {
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ResourceFlags: u32 {
        // for image resource flip vertical
        const flip_v = 1;
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum FaceCullMode {
    #[default]
    None = 0x0,
    Front = 0x1,
    Back = 0x2,
    FrontAndBack = 0x3,
}

#[derive(Clone)]
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

#[derive(Default, Debug, Clone, Copy, PartialEq)]
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
    #[default]
    Unknown,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
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
    #[default]
    Unknown,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum ShaderScope {
    Global = 0,
    Instance = 1,
    Local = 2,
    #[default]
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
    pub face_cull_mode: FaceCullMode,
    pub attributes: Vec<ShaderAttributeConfig>,
    pub uniforms: Vec<ShaderUniformConfig>,
    pub renderpass_name: String,
    pub stages: Vec<ShaderStage>,
    pub stage_filenames: Vec<String>,
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self {
            name: Default::default(),
            face_cull_mode: Default::default(),
            attributes: Default::default(),
            uniforms: Default::default(),
            renderpass_name: Default::default(),
            stages: Default::default(),
            //stage_names: Default::default(),
            stage_filenames: Default::default(),
        }
    }
}
