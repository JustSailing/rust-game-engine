use crate::application::{
    basic::math::consts::INVALID_ID,
    basic::math::{matrix4::Matrix4, vec4::Vec4},
    renderer::vulkan::vulkan_backend::VulkanRenderPass,
    resources::resource_types::{Geometry, Texture},
};
// this can be temporary
use ash::vk::Framebuffer;

use std::{cell::RefCell, rc::Rc};

use bitflags::bitflags;

pub enum RendererBackendType {
    Vulkan,
    OpenGL,
    DirectX,
}

#[derive(Clone)]
pub struct GeometryRenderData {
    pub model: Matrix4,
    pub geometry: Rc<RefCell<Geometry>>,
}

#[repr(C)]
pub struct RendererPacket {
    pub delta_time: f32,
    pub geometries: Vec<GeometryRenderData>,
    pub ui_geometries: Vec<GeometryRenderData>,
}

#[derive(Debug, Copy, Clone)]
pub enum RendererDebugViewMode {
    Default = 0,
    Lighting = 1,
    Normals = 2,
}

bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RenderpassClearFlags: u32 {
    const ColourBuffer = 1 << 1;
    const DepthBuffer = 1 << 2;
    const StencilBuffer = 1 << 3;
    }
}

#[derive(Debug, Default, Clone)]
pub struct RenderpassConfig {
    pub name: String,
    pub prev_name: String,
    pub next_name: String,
    pub render_area: Vec4,
    pub clear_color: Vec4,
    pub clear_flags: RenderpassClearFlags,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Renderpass {
    pub id: usize,
    pub render_area: Vec4,
    pub clear_colour: Vec4,
    pub clear_flags: RenderpassClearFlags,
    pub targets: Vec<RenderTarget>,
    pub internal_data: VulkanRenderPass,
}

impl Default for Renderpass {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            render_area: Vec4::new_zeroes(),
            clear_colour: Vec4::new_zeroes(),
            clear_flags: RenderpassClearFlags::empty(),
            targets: Default::default(),
            internal_data: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct RenderTarget {
    pub sync_to_window: u32,
    pub attachments: Vec<Rc<RefCell<Texture>>>,
    pub internal_framebuffer: Framebuffer,
}

#[derive(Debug, Clone)]
pub struct RendererBackendConfig {
    pub application_name: String,
    pub renderpass_configs: Vec<RenderpassConfig>,
    // Although in the Kohi game engine this is here. Leting the backend handle this
    // is more in line with how the architecture of the game engine is now might change later
    // pub on_rendertarget_refresh_required: fn() -> (),
}
