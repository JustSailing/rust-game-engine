use crate::application::{
    basic::{
        math::{
            consts::{INVALID_ID, deg_to_rad},
            matrix4::Matrix4,
            vec3::Vec3,
            vec4::Vec4,
        },
        window::Window,
    },
    renderer::vulkan::vulkan_backend::{BuiltInRenderpass, VulkanBackendError, VulkanContext},
    resources::resource_types::{
        Geometry, Resource, ResourceData, ShaderConfig, ShaderStage, Texture,
    },
    systems::{
        resource_system::{ResourceSysError, ResourceSystem},
        shader_system::Shader,
    },
};

use std::{cell::RefCell, ffi::c_void, rc::Rc};

use thiserror::Error;

type Result<T> = std::result::Result<T, RendererError>;

pub enum RendererBackendType {
    Vulkan,
    OpenGL,
    DirectX,
}

#[derive(Copy, Clone)]
#[repr(C, align(16))] // both C and align are needed or some funky stuff happens
pub struct MaterialGlobalUBO {
    pub projection: Matrix4,
    pub view: Matrix4,
    pub padding: [Matrix4; 2], // for NVidia cards
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct MaterialInstanceUBO {
    pub diffuse_color: Vec4,
    pub padding: [Vec4; 3],
}

#[derive(Copy, Clone)]
#[repr(C, align(16))] // both C and align are needed or some funky stuff happens
pub struct UIglobalUBO {
    pub projection: Matrix4,
    pub view: Matrix4,
    pub padding: [Matrix4; 2], // for NVidia cards
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct UIinstanceUBO {
    pub diffuse_color: Vec4,
    pub padding: [Vec4; 3],
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
    #[error("frontend renderer error: shader system error {file} {line}")]
    ShaderSysError { file: &'static str, line: u32 },
    #[error("frontend renderer error: material system error {file} {line}")]
    MaterialSysError { file: &'static str, line: u32 },
}

#[repr(C)]
pub struct Renderer {
    backend: VulkanContext,
    resource_system: Rc<RefCell<ResourceSystem>>,
    pub projection: Matrix4,
    pub view: Matrix4,
    pub view_position: Vec3,
    pub ambient_colour: Vec4,
    pub ui_projection: Matrix4,
    pub ui_view: Matrix4,
    far_clip: f32,
    near_clip: f32,
    material_shader_id: u32,
    ui_shader_id: u32,
    frame_number: u64,
}

impl Renderer {
    pub fn initialize(
        app_name: &str,
        window: &Window,
        resource_system: Rc<RefCell<ResourceSystem>>,
    ) -> Result<Self> {
        let backend = VulkanContext::initialize(app_name, window, resource_system.clone())
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(Self {
            backend: backend,
            projection: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 100.0),
            view: Matrix4::inverse(&Matrix4::translation(&Vec3::new(0.0, 0.0, 30.0))),
            view_position: Vec3::new_zeroes(),
            ambient_colour: Vec4::new(0.25, 0.25, 0.25, 1.0),
            ui_projection: Matrix4::orthographic(0.0, 1280.0, 720.0, 0.0, -100.0, 100.0),
            ui_view: Matrix4::inverse(&Matrix4::identity()),
            far_clip: 1000.0,
            near_clip: 0.1,
            material_shader_id: INVALID_ID as u32,
            ui_shader_id: INVALID_ID as u32,
            frame_number: 0,
            resource_system: resource_system,
        })
    }

    pub fn create_texture(&self, pixels: &[u8], texture: &mut Texture) -> Result<()> {
        self.backend.create_texture(pixels, texture).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })
    }

    pub fn set_default_texture(&mut self, texture: Rc<RefCell<Texture>>) -> Result<()> {
        self.backend
            .set_default_texture(texture)
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
    pub fn get_renderpass_id(&self, name: &String) -> Result<BuiltInRenderpass> {
        match name.as_str() {
            "Renderpass.Builtin.World" => Ok(BuiltInRenderpass::World),
            "Renderpass.Builtin.UI" => Ok(BuiltInRenderpass::UI),
            _ => {
                return Err(RendererError::RendererIdInvalid {
                    file: file!(),
                    line: line!(),
                });
            }
        }
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

    pub fn begin_renderpass(&mut self, renderpass_id: BuiltInRenderpass) -> Result<()> {
        self.backend.begin_renderpass(renderpass_id).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        Ok(())
    }

    pub fn end_renderpass(&mut self, renderpass_id: BuiltInRenderpass) -> Result<()> {
        self.backend.end_renderpass(renderpass_id).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        Ok(())
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

    pub fn draw_geometry(&mut self, data: &mut Vec<GeometryRenderData>, _delta: f32) -> Result<()> {
        for geo in data.iter_mut() {
            self.backend
                .draw_geometry(geo)
                .map_err(|e| RendererError::BackendRendererError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        }
        Ok(())
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

    pub fn set_view(&mut self, view: Matrix4, view_position: Vec3) -> Result<()> {
        self.view = view;
        self.view_position = view_position;
        Ok(())
    }

    pub fn shader_create(
        &mut self,
        shader: &mut Shader,
        renderpass_id: BuiltInRenderpass,
        stage_count: u8,
        stage_filenames: &Vec<String>,
        stages: &Vec<ShaderStage>,
    ) -> Result<()> {
        self.backend
            .shader_create(shader, renderpass_id, stage_count, stage_filenames, stages)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }
    pub fn shader_destroy(&self, shader: &mut Shader) -> Result<()> {
        self.backend
            .shader_destroy(shader)
            .map_err(|e| RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok(())
    }

    pub fn shader_use(&self, shader: &Shader) -> Result<()> {
        self.backend
            .shader_use(shader)
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
    pub fn shader_acquire_instance_resources(&self, shader: &mut Shader) -> Result<u32> {
        self.backend
            .shader_acquire_instance_resources(shader)
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
            ResourceData::Unknown => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "ShaderResourceData",
                    given: "Unknown",
                    file: file!(),
                    line: line!(),
                });
            }

            ResourceData::ImageResourceData(_) => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "ShaderResourceData",
                    given: "ImageResourceData",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::MaterialResourceData(_) => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "ShaderResourceData",
                    given: "MaterialResourceData",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::BinaryResourceData(_) => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "ShaderResourceData",
                    given: "BinaryResourceData",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::ShaderResourceData(ref shader_config) => return Ok(shader_config),
        };
    }
}
