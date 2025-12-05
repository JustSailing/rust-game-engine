use crate::{
    basic::{
        math::{consts::INVALID_ID, vec4::Vec4},
        window::Window,
    },
    renderer::{
        renderer_types::{
            GeometryRenderData, RendererBackendConfig, Renderpass, RenderpassClearFlags,
            RenderpassConfig, RenderpassHandle,
        },
        vulkan::vulkan_backend::{VulkanBackendError, VulkanContext},
    },
    resources::resource_types::{
        Geometry, Resource, ResourceData, ShaderConfig, ShaderStage, Texture, TextureHandle,
        TextureMap,
    },
    systems::{
        camera_system::{CameraHandle, CameraSysError, CameraSystem, DEFAULT_CAMERA_NAME},
        geometry_system::GeometrySysError,
        material_system::MaterialSysError,
        render_view_system::{RenderViewSysError, RenderViewSystem},
        resource_system::{ResourceSysError, ResourceSystem},
        shader_system::{Shader, ShaderSysError},
        texture_system::TextureSystem,
    },
};

use std::{cell::RefCell, ffi::c_void, rc::Rc};

use thiserror::Error;

type Result<T> = std::result::Result<T, RendererError>;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("frontend renderer error: already initialized {} {}", file, line)]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("frontend renderer error: already shutdown {} {}", file, line)]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("frontend renderer error: not initialized {} {}", file, line)]
    NotInitialized { file: &'static str, line: u32 },
    #[error("frontend renderer error: renderpass id is not recognized {file} {line}")]
    RendererIdInvalid { file: &'static str, line: u32 },
    #[error(
        "frontend renderer error: wrong resource type given: {given}, expected: {expected} {file} {line}"
    )]
    WrongResourceDataType {
        expected: &'static str,
        given: &'static str,
        file: &'static str,
        line: u32,
    },
    #[error("frontend renderer error: backend renderer error: {0}")]
    BackendRendererErr(#[from] VulkanBackendError),
    #[error("frontend renderer error: resource system error: {0}")]
    ResourceSysErr(#[from] ResourceSysError),
    #[error("frontend renderer error: camera_system error: {0}")]
    CameraSysErr(#[from] CameraSysError),
    #[error("frontend renderer error: shader system error {0}")]
    ShaderSysErr(#[from] ShaderSysError),
    #[error("frontend renderer error: material system error {0}")]
    MaterialSysErr(#[from] MaterialSysError),

    #[error("frontend renderer error: geometry system error {0}")]
    GeometrySysErr(#[from] GeometrySysError),
    #[error("frontend renderer error: render view system error {0}")]
    RenderViewSysErr(#[from] RenderViewSysError),
}

#[repr(C)]
pub struct Renderer<'a> {
    backend: VulkanContext<'a>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    camera_system: Rc<RefCell<CameraSystem>>,
    render_view_system: Option<Rc<RefCell<RenderViewSystem<'a>>>>,
    camera: CameraHandle,
    material_shader_id: u32,
    ui_shader_id: u32,
    window_render_target_count: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    resizing: bool,
    frames_since_resize: u32,
    pub frame_number: u64,
}

impl<'a> Renderer<'a> {
    pub fn initialize(
        app_name: &str,
        window: &Window,
        resource_system: Rc<RefCell<ResourceSystem>>,
        texture_system: Rc<RefCell<TextureSystem<'a>>>,
        camera_system: Rc<RefCell<CameraSystem>>,
    ) -> Result<Self> {
        let camera_handle = camera_system
            .borrow_mut()
            .acquire(DEFAULT_CAMERA_NAME, true)?;

        let world_renderpass_name = String::from("Renderpass.Builtin.World");
        let ui_renderpass_name = String::from("Renderpass.Builtin.UI");
        let world_renderpass_clear_flags: RenderpassClearFlags = RenderpassClearFlags::ColourBuffer
            | RenderpassClearFlags::DepthBuffer
            | RenderpassClearFlags::StencilBuffer;

        let world_renderpass_config = RenderpassConfig {
            name: world_renderpass_name.clone(),
            prev_name: String::from(""),
            next_name: ui_renderpass_name.clone(),
            render_area: Vec4::new(0.0, 0.0, 1280.0, 720.0),
            clear_color: Vec4::new(0.0, 0.0, 0.2, 1.0),
            clear_flags: world_renderpass_clear_flags,
        };

        let ui_renderpass_config = RenderpassConfig {
            name: ui_renderpass_name.clone(),
            prev_name: world_renderpass_name.clone(),
            next_name: String::from(""),
            render_area: Vec4::new(0.0, 0.0, 1280.0, 720.0),
            clear_color: Vec4::new(0.0, 0.0, 0.2, 1.0),
            clear_flags: RenderpassClearFlags::empty(),
        };

        let renderpass_configs = vec![world_renderpass_config, ui_renderpass_config];
        let renderpass_backend_config = RendererBackendConfig {
            application_name: app_name.to_string(),
            renderpass_configs,
        };

        let mut window_render_target_count = 0;

        let backend = VulkanContext::initialize(
            window,
            &renderpass_backend_config,
            &mut window_render_target_count,
            Rc::clone(&resource_system),
            texture_system,
        )?;

        Ok(Self {
            backend,
            camera: camera_handle,
            camera_system: camera_system,
            material_shader_id: INVALID_ID as u32,
            ui_shader_id: INVALID_ID as u32,
            frame_number: 0,
            resource_system,
            render_view_system: None,
            resizing: false,
            framebuffer_width: 1280,
            framebuffer_height: 800,
            frames_since_resize: 0,
            window_render_target_count: window_render_target_count,
        })
    }

    pub fn set_render_view_system(
        &mut self,
        render_view_system: Rc<RefCell<RenderViewSystem<'a>>>,
    ) {
        self.render_view_system = Some(render_view_system);
    }

    pub fn create_texture(&self, name: &str, pixels: &[u8], texture: &mut Texture) -> Result<()> {
        self.backend.create_texture(name, pixels, texture)?;
        Ok(())
    }

    pub fn create_writable_texture(&self, texture: &mut Texture) -> Result<()> {
        Ok(self.backend.create_writable_texture(texture)?)
    }

    pub fn write_data_texture(
        &self,
        texture: &mut Texture,
        offset: u32,
        size: u64,
        pixels: &[u8],
    ) -> Result<()> {
        self.backend
            .write_data_texture(texture, offset, size, pixels)?;
        Ok(())
    }

    pub fn resize_texture(&self, texture: &mut Texture, width: u32, height: u32) -> Result<()> {
        self.backend.resize_texture(texture, width, height)?;
        Ok(())
    }

    pub fn destroy_texture(&self, texture: &Texture) -> Result<()> {
        self.backend.destroy_texture(&texture)?;
        Ok(())
    }

    pub fn create_geometry<T: Clone, U: Clone>(
        &mut self,
        geometry: &mut Geometry,
        vertices: &[T],
        indicies: &[U],
    ) -> Result<()> {
        self.backend.create_geometry(geometry, vertices, indicies)?;
        Ok(())
    }

    pub fn destroy_geometry(&mut self, geometry: &Geometry) -> Result<()> {
        self.backend.destroy_geometry(geometry)?;
        Ok(())
    }

    pub fn begin_frame(&mut self, delta: f32) -> Result<bool> {
        Ok(self.backend.begin_frame(delta)?)
    }

    pub fn end_frame(&mut self, delta: f32) -> Result<()> {
        self.backend.end_frame(delta)?;
        self.frame_number += 1;
        Ok(())
    }

    pub fn draw_geometry(&mut self, data: &mut GeometryRenderData, _delta: f32) -> Result<()> {
        self.frame_number += 1;
        self.backend.draw_geometry(data)?;
        Ok(())
    }

    pub fn begin_renderpass_by_name(&mut self, renderpass_name: &str) -> Result<()> {
        self.backend.begin_renderpass_by_name(renderpass_name)?;
        Ok(())
    }

    pub fn begin_renderpass_by_id(&mut self, renderpass_id: RenderpassHandle) -> Result<()> {
        self.backend.begin_renderpass_by_id(renderpass_id)?;
        Ok(())
    }

    pub fn end_renderpass(&mut self) -> Result<()> {
        self.backend.end_renderpass()?;
        Ok(())
    }

    pub fn get_renderpass_handle(&self, name: &str) -> Result<RenderpassHandle> {
        Ok(self.backend.get_renderpass_handle(name)?)
    }
    pub fn get_renderpass_by_name(&self, name: &str) -> Result<&Renderpass> {
        Ok(self.backend.get_renderpass_by_name(name)?)
    }

    pub fn get_mut_renderpass_by_name(&mut self, name: &str) -> Result<&mut Renderpass> {
        Ok(self.backend.get_mut_renderpass_by_name(name)?)
    }

    pub fn get_renderpass_by_id(&self, id: usize) -> Result<&Renderpass> {
        Ok(self.backend.get_renderpass_by_id(id)?)
    }

    pub fn get_mut_renderpass_by_id(&mut self, id: usize) -> Result<&mut Renderpass> {
        Ok(self.backend.get_mut_renderpass_by_id(id)?)
    }
    pub fn on_resize(&mut self, width: i32, height: i32) -> Result<()> {
        self.backend.on_resize(width, height)?;
        Ok(())
    }

    pub fn create_shader(
        &mut self,
        shader: &mut Shader<'a>,
        renderpass_name: &str,
        stage_count: u8,
        stage_filenames: &Vec<String>,
        stages: &Vec<ShaderStage>,
    ) -> Result<()> {
        self.backend.create_shader(
            shader,
            renderpass_name,
            stage_count,
            stage_filenames,
            stages,
        )?;
        Ok(())
    }
    pub fn destroy_shader(&self, shader: &mut Shader) -> Result<()> {
        self.backend.destroy_shader(shader)?;
        Ok(())
    }

    pub fn use_shader(&self, shader: &Shader) -> Result<()> {
        self.backend.use_shader(shader)?;
        Ok(())
    }
    pub fn bind_globals_for_shader(&self, shader: &mut Shader) -> Result<()> {
        self.backend.bind_globals_for_shader(shader)?;
        Ok(())
    }
    pub fn bind_instance_for_shader(&self, shader: &mut Shader) -> Result<()> {
        self.backend
            .bind_instance_for_shader(shader, shader.bound_instance_id as u32)?;
        Ok(())
    }
    pub fn apply_globals_for_shader(&self, shader: &mut Shader) -> Result<()> {
        self.backend.apply_globals_for_shader(shader)?;
        Ok(())
    }
    pub fn apply_instance_for_shader(&self, shader: &mut Shader) -> Result<()> {
        self.backend.apply_instance_for_shader(shader)?;
        Ok(())
    }
    pub fn set_default_texture(&mut self, default_texture: TextureHandle) -> Result<()> {
        self.backend.set_default_texture(default_texture)?;
        Ok(())
    }

    pub fn acquire_texture_map_resources(&self, map: &mut TextureMap) -> Result<()> {
        self.backend.acquire_texture_map_resources(map)?;
        Ok(())
    }

    pub fn release_texture_map_resources(&self, map: &mut TextureMap) -> Result<()> {
        self.backend.release_texture_map_resources(map)?;
        Ok(())
    }

    pub fn acquire_shader_instance_resources(
        &self,
        shader: &mut Shader,
        maps: &Vec<&TextureMap>,
    ) -> Result<u32> {
        Ok(self
            .backend
            .acquire_shader_instance_resources(shader, maps)?)
    }
    pub fn release_shader_instance_resources(
        &self,
        shader: &mut Shader,
        instance_id: u32,
    ) -> Result<()> {
        self.backend
            .release_shader_instance_resources(shader, instance_id)?;
        Ok(())
    }
    pub fn set_uniform_for_shader(
        &self,
        shader: &mut Shader,
        uniform_index: usize,
        value: *const c_void,
    ) -> Result<()> {
        self.backend
            .set_uniform_for_shader(shader, uniform_index, value)?;
        Ok(())
    }

    fn destroy_renderer_backend(&self) -> Result<()> {
        Ok(self.backend.shutdown()?)
    }

    fn get_shader_config(resource: &Resource) -> Result<&ShaderConfig> {
        match resource.data {
            ResourceData::ShaderResourceData(ref shader_config) => Ok(shader_config),
            ResourceData::Unknown => Err(RendererError::WrongResourceDataType {
                expected: "ShaderResourceData",
                given: "Unknown",
                file: file!(),
                line: line!(),
            }),

            ResourceData::ImageResourceData(_) => Err(RendererError::WrongResourceDataType {
                expected: "ShaderResourceData",
                given: "ImageResourceData",
                file: file!(),
                line: line!(),
            }),
            ResourceData::MaterialResourceData(_) => Err(RendererError::WrongResourceDataType {
                expected: "ShaderResourceData",
                given: "MaterialResourceData",
                file: file!(),
                line: line!(),
            }),
            ResourceData::BinaryResourceData(_) => Err(RendererError::WrongResourceDataType {
                expected: "ShaderResourceData",
                given: "BinaryResourceData",
                file: file!(),
                line: line!(),
            }),
            ResourceData::MeshResourceData(_) => Err(RendererError::WrongResourceDataType {
                expected: "ShaderResourceData",
                given: "MeshResourceData",
                file: file!(),
                line: line!(),
            }),
        }
    }
}
