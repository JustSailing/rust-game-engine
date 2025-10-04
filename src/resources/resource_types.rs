use crate::application::{
    basic::math::{consts::INVALID_ID, vec4::Vec4},
    renderer::vulkan::vulkan_image::VulkanImage,
};
use ash::vk::Sampler;

#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
pub struct TextureData {
    pub image: VulkanImage,
    pub sampler: Sampler,
}

#[derive(Clone, Copy)]
pub enum TextureUse {
    Unknown = 0x00,
    MapDiffuse = 0x01,
}

pub struct TextureMap<'a> {
    pub texture: Option<&'a mut Texture>,
    pub use_type: TextureUse,
}

pub struct Material<'a> {
    pub id: usize,
    pub generation: usize,
    pub internal_id: usize,
    pub name: String,
    pub diffuse_colour: Vec4,
    pub diffuse_map: TextureMap<'a>,
}

impl<'a> Default for Material<'_> {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            generation: INVALID_ID,
            internal_id: INVALID_ID,
            name: Default::default(),
            diffuse_colour: Vec4::new_ones(),
            diffuse_map: unsafe { std::mem::zeroed() },
        }
    }
}
