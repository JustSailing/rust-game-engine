use crate::application::{
    basic::{
        math::{
            consts::{INVALID_ID, deg_to_rad},
            matrix4::Matrix4,
            vec4::Vec4,
        },
        window::Window,
    },
    renderer::{
        renderer_types::{
            GeometryRenderData, RendererBackendConfig, RendererDebugViewMode, RendererPacket,
            Renderpass, RenderpassClearFlags, RenderpassConfig,
        },
        vulkan::vulkan_backend::{VulkanBackendError, VulkanContext},
    },
    resources::resource_types::{
        Geometry, Resource, ResourceData, ShaderConfig, ShaderStage, Texture, TextureHandle,
        TextureMap,
    },
    systems::{
        camera_system::{CameraHandle, CameraSysError, CameraSystem},
        resource_system::{ResourceSysError, ResourceSystem},
        shader_system::Shader,
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
        "{source}\nfrontend renderer error: backend renderer error {} {}",
        file,
        line
    )]
    BackendRendererError {
        source: VulkanBackendError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nfrontend renderer error: resource system error {file} {line}")]
    ResouceSysError {
        source: ResourceSysError,
        file: &'static str,
        line: u32,
    },
    #[error(
        "frontend renderer error: wrong resource type given: {given}, expected: {expected} {file} {line}"
    )]
    WrongResourceDataType {
        expected: &'static str,
        given: &'static str,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nfrontend renderer error: camera_system error {file} {line}")]
    CameraSysError {
        source: CameraSysError,
        file: &'static str,
        line: u32,
    },
    #[error("frontend renderer error: shader system error {file} {line}")]
    ShaderSysError { file: &'static str, line: u32 },
    #[error("frontend renderer error: material system error {file} {line}")]
    MaterialSysError { file: &'static str, line: u32 },
}

#[repr(C)]
pub struct Renderer {
    backend: VulkanContext,
    resource_system: Rc<RefCell<ResourceSystem>>,
    camera_system: Rc<RefCell<CameraSystem>>,
    camera: CameraHandle,
    pub projection: Matrix4,
    pub ambient_colour: Vec4,
    pub ui_projection: Matrix4,
    pub ui_view: Matrix4,
    far_clip: f32,
    near_clip: f32,
    material_shader_id: u32,
    ui_shader_id: u32,
    pub render_mode: RendererDebugViewMode,
    window_render_target_count: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    resizing: bool,
    frames_since_resize: u32,
    pub frame_number: u64,
}

impl Renderer {
    pub fn initialize(
        app_name: &str,
        window: &Window,
        resource_system: Rc<RefCell<ResourceSystem>>,
        texture_system: Rc<RefCell<TextureSystem>>,
        camera_system: Rc<RefCell<CameraSystem>>,
    ) -> Result<Self> {
        let camera_handle = camera_system
            .borrow_mut()
            .acquire("default_texture", true)
            .map_err(|e| RendererError::CameraSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

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
        )
        .map_err(|e| RendererError::BackendRendererError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        Ok(Self {
            backend,
            camera: camera_handle,
            camera_system: camera_system,
            projection: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 100.0),
            ambient_colour: Vec4::new(0.25, 0.25, 0.25, 1.0),
            ui_projection: Matrix4::orthographic(0.0, 1280.0, 720.0, 0.0, -100.0, 100.0),
            ui_view: Matrix4::inverse(&Matrix4::identity()),
            far_clip: 1000.0,
            near_clip: 0.1,
            material_shader_id: INVALID_ID as u32,
            ui_shader_id: INVALID_ID as u32,
            render_mode: RendererDebugViewMode::Default,
            frame_number: 0,
            resource_system,
            resizing: false,
            framebuffer_width: 1280,
            framebuffer_height: 800,
            frames_since_resize: 0,
            window_render_target_count: window_render_target_count,
        })
    }

    pub fn create_texture(&self, name: &str, pixels: &[u8], texture: &mut Texture) -> Result<()> {
        self.backend
            .create_texture(name, pixels, texture)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn create_writable_texture(&self, texture: &mut Texture) -> Result<()> {
        self.backend.create_writable_texture(texture).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })
    }

    pub fn write_data_texture(
        &self,
        texture: &mut Texture,
        offset: u32,
        size: u64,
        pixels: &[u8],
    ) -> Result<()> {
        self.backend
            .write_data_texture(texture, offset, size, pixels)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn resize_texture(&self, texture: &mut Texture, width: u32, height: u32) -> Result<()> {
        self.backend
            .resize_texture(texture, width, height)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn destroy_texture(&self, texture: &Texture) -> Result<()> {
        self.backend
            .destroy_texture(&texture)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn create_geometry<T: Clone, U: Clone>(
        &mut self,
        geometry: &mut Geometry,
        vertices: &[T],
        indicies: &[U],
    ) -> Result<()> {
        self.backend
            .create_geometry(geometry, vertices, indicies)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn destroy_geometry(&mut self, geometry: &Geometry) -> Result<()> {
        self.backend
            .destroy_geometry(geometry)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn begin_frame(&mut self, packet: &mut RendererPacket) -> Result<bool> {
        self.backend.begin_frame(packet.delta_time).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })
    }

    pub fn end_frame(&mut self, delta: f32) -> Result<()> {
        self.backend
            .end_frame(delta)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        self.frame_number += 1;
        Ok(())
    }

    pub fn draw_geometry(&mut self, data: &mut GeometryRenderData, _delta: f32) -> Result<()> {
        self.frame_number += 1;
        self.backend
            .draw_geometry(data)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(())
    }

    pub fn begin_renderpass(&mut self, renderpass_name: &str) -> Result<()> {
        self.backend
            .begin_renderpass(renderpass_name)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }

    pub fn end_renderpass(&mut self) -> Result<()> {
        self.backend
            .end_renderpass()
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }
    pub fn get_renderpass(&self, name: &str) -> Result<Rc<RefCell<Renderpass>>> {
        self.backend
            .get_renderpass(name)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn on_resize(&mut self, width: i32, height: i32) -> Result<()> {
        self.projection = Matrix4::perspective(
            45.0,
            width as f32 / height as f32,
            self.near_clip,
            self.far_clip,
        );
        self.ui_projection =
            Matrix4::orthographic(0.0, width as f32, height as f32, 0.0, -100.0, 100.0);
        self.backend
            .on_resize(width, height)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn set_render_mode(&mut self, render_mode: u32) -> Result<()> {
        match render_mode {
            0 => self.render_mode = RendererDebugViewMode::Default,
            1 => self.render_mode = RendererDebugViewMode::Lighting,
            2 => self.render_mode = RendererDebugViewMode::Normals,
            // should warn here
            _ => self.render_mode = RendererDebugViewMode::Default,
        }

        Ok(())
    }

    pub fn create_shader(
        &mut self,
        shader: &mut Shader,
        renderpass_name: &str,
        stage_count: u8,
        stage_filenames: &Vec<String>,
        stages: &Vec<ShaderStage>,
    ) -> Result<()> {
        self.backend
            .create_shader(
                shader,
                renderpass_name,
                stage_count,
                stage_filenames,
                stages,
            )
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }
    pub fn destroy_shader(&self, shader: &mut Shader) -> Result<()> {
        self.backend
            .destroy_shader(shader)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(())
    }

    pub fn use_shader(&self, shader: &Shader) -> Result<()> {
        self.backend
            .use_shader(shader)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(())
    }
    pub fn shader_bind_globals(&self, shader: &mut Shader) -> Result<()> {
        self.backend.shader_bind_globals(shader).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        Ok(())
    }
    pub fn shader_bind_instance(&self, shader: &mut Shader) -> Result<()> {
        self.backend
            .shader_bind_instance(shader, shader.bound_instance_id as u32)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }
    pub fn shader_apply_globals(&self, shader: &mut Shader) -> Result<()> {
        self.backend.shader_apply_globals(shader).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        Ok(())
    }
    pub fn shader_apply_instance(&self, shader: &mut Shader) -> Result<()> {
        self.backend.shader_apply_instance(shader).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        Ok(())
    }
    pub fn set_default_texture(&mut self, default_texture: TextureHandle) -> Result<()> {
        self.backend
            .set_default_texture(default_texture)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }

    pub fn texture_map_acquire_resources(&self, map: &mut TextureMap) -> Result<()> {
        self.backend
            .texture_map_acquire_resources(map)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }

    pub fn texture_map_release_resources(&self, map: &mut TextureMap) -> Result<()> {
        self.backend
            .texture_map_release_resources(map)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }

    pub fn shader_acquire_instance_resources(
        &self,
        shader: &mut Shader,
        maps: &Vec<&TextureMap>,
    ) -> Result<u32> {
        self.backend
            .shader_acquire_instance_resources(shader, maps)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }
    pub fn shader_release_instance_resources(
        &self,
        shader: &mut Shader,
        instance_id: u32,
    ) -> Result<()> {
        self.backend
            .shader_release_instance_resources(shader, instance_id)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(())
    }
    pub fn set_uniform(
        &self,
        shader: &mut Shader,
        uniform_index: usize,
        value: *const c_void,
    ) -> Result<()> {
        self.backend
            .set_uniform(shader, uniform_index, value)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(())
    }

    fn destroy_renderer_backend(&self) -> Result<()> {
        Ok(self
            .backend
            .shutdown()
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?)
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
