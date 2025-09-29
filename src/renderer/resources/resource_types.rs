use super::super::vulkan::vulkan_image::VulkanImage;
use ash::vk::Sampler;

#[derive(Clone, Copy)]
pub struct Texture {
    pub id: usize,
    pub width: u32,
    pub height: u32,
    pub channel_count: u8,
    pub has_transparency: bool,
    pub generation: u32,
    pub internal_data: TextureData,
}
#[derive(Clone, Copy)]
pub struct TextureData {
    pub image: VulkanImage,
    pub sampler: Sampler,
}