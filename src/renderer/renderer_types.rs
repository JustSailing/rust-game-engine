use crate::{
    basic::{
        event::EventCtx,
        math::{consts::INVALID_ID, matrix4::Matrix4, vec3::Vec3, vec4::Vec4},
    },
    renderer::{
        frontend_renderer::{Renderer, RendererError},
        vulkan::vulkan_backend::VulkanRenderPass,
    },
    resources::resource_types::{GeometryHandle, Mesh, Skybox, TextureHandle},
    systems::{
        camera_system::CameraSystem, geometry_system::GeometrySystem,
        material_system::MaterialSystem, shader_system::ShaderSystem,
        texture_system::TextureSystem,
    },
};
use std::{cell::RefCell, rc::Rc};
// this can be temporary
use ash::vk::Framebuffer;
use bitflags::bitflags;

#[derive(Default, Debug, Clone, Copy)]
pub enum RendererBackendType {
    #[default]
    Vulkan,
    OpenGL,
    DirectX,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GeometryRenderData {
    pub model: Matrix4,
    pub geometry_handle: GeometryHandle,
}

#[repr(C)]
#[derive(Default)]
pub struct RendererPacket {
    pub delta_time: f32,
    pub views: Vec<RenderViewPacket>,
}

#[repr(C)]
#[derive(Clone)]
pub struct MeshPacketData {
    pub meshes: Vec<Mesh>,
}

#[repr(C)]
#[derive(Clone)]
pub struct SkyboxPacketData {
    pub skybox: Skybox,
}

#[repr(C)]
//#[derive(Clone)]
pub enum PacketData<'a> {
    Skybox(&'a mut SkyboxPacketData),
    Mesh(&'a mut MeshPacketData),
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub enum RendererDebugViewMode {
    #[default]
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

#[repr(C)]
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

pub type RenderpassHandle = usize;

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct RenderTarget {
    pub sync_to_window: u32,
    pub attachments: Vec<TextureHandle>,
    pub internal_framebuffer: Framebuffer,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct RendererBackendConfig {
    pub application_name: String,
    pub renderpass_configs: Vec<RenderpassConfig>,
    // Although in the Kohi game engine this is here. Leting the backend handle this
    // is more in line with how the architecture of the game engine is now might change later
    // pub on_rendertarget_refresh_required: fn() -> (),
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub enum RenderViewKnownType {
    #[default]
    Unknown = 0x0,
    World = 0x1,
    UI = 0x2,
    Skybox = 0x3,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub enum RenderViewMatrixViewSource {
    #[default]
    Unknown = 0x0,
    SceneCamera = 0x1,
    UICamera = 0x2,
    LightCamera = 0x3,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub enum RenderViewProjectionMatrixSource {
    #[default]
    Unknown = 0x0,
    Perspective = 0x1,
    Orthographic = 0x2,
}

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct RenderViewPassConfig {
    pub name: String,
}

impl RenderViewPassConfig {
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
}

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct RenderViewConfig {
    pub name: String,
    pub custom_shader_name: String,
    pub width: u16,
    pub height: u16,
    pub known_type: RenderViewKnownType,
    pub view_matrix_source: RenderViewMatrixViewSource,
    pub projection_matrix_source: RenderViewProjectionMatrixSource,
    pub passes: Vec<RenderViewPassConfig>,
}

impl RenderViewConfig {
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn custom_shader_name(mut self, custom_shader_name: &str) -> Self {
        self.custom_shader_name = custom_shader_name.to_string();
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn known_type(mut self, known_type: RenderViewKnownType) -> Self {
        self.known_type = known_type;
        self
    }

    pub fn view_matrix_source(mut self, view_matrix_source: RenderViewMatrixViewSource) -> Self {
        self.view_matrix_source = view_matrix_source;
        self
    }

    pub fn projection_matrix_source(
        mut self,
        projection_matrix_source: RenderViewProjectionMatrixSource,
    ) -> Self {
        self.projection_matrix_source = projection_matrix_source;
        self
    }

    pub fn passes(mut self, passes: Vec<RenderViewPassConfig>) -> Self {
        self.passes = passes;
        self
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct RenderViewPacket {
    pub view_matrix: Matrix4,
    pub projection_matrix: Matrix4,
    pub view_position: Vec3,
    pub ambient_colour: Vec4,
    pub data: Vec<GeometryRenderData>,
    pub extended: Option<Skybox>,
    pub custom_shader_name: String,
}

pub trait RenderView {
    fn create(
        &mut self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        camera_system: &Rc<RefCell<CameraSystem>>,
    ) -> Result<(), RendererError>;
    fn get_id(&self) -> usize;
    fn resize(
        &mut self,
        width: u16,
        height: u16,
        renderer_system: &Rc<RefCell<Renderer>>,
    ) -> Result<(), RendererError>;
    fn build_packet(
        &self,
        packet: &mut PacketData,
        camera_system: &Rc<RefCell<CameraSystem>>,
        geometry_system: &Rc<RefCell<GeometrySystem>>,
        material_system: &Rc<RefCell<MaterialSystem>>,
        texture_system: &Rc<RefCell<TextureSystem>>,
    ) -> Result<RenderViewPacket, RendererError>;
    fn render(
        &self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        material_system: &Rc<RefCell<MaterialSystem>>,
        renderer_system: &Rc<RefCell<Renderer>>,
        geometry_system: &Rc<RefCell<GeometrySystem>>,
        frame_number: u64,
        render_view_packet: &mut RenderViewPacket,
    ) -> Result<(), RendererError>;
    fn handle_event(
        &mut self,
        code: usize,
        _sender: *const std::os::raw::c_void,
        _listener: *const std::os::raw::c_void,
        data: &EventCtx,
    ) -> bool;
}
