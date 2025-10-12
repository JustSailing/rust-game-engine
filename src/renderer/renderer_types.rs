use crate::application::{
    basic::{
        math::{consts::deg_to_rad, matrix4::Matrix4, vec3::Vec3, vec4::Vec4},
        window::Window,
    },
    renderer::vulkan::vulkan_backend::{BuiltInRenderpass, VulkanBackendError, VulkanContext},
    resources::resource_types::{Geometry, Material, Texture},
};

use std::{cell::RefCell, rc::Rc};

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

pub struct GeometryRenderData<'a> {
    pub model: Matrix4,
    pub geometry: Rc<RefCell<&'a mut Geometry<'a>>>,
}

#[repr(C)]
pub struct RendererPacket<'a> {
    pub delta_time: f32,
    pub geometries: Vec<GeometryRenderData<'a>>,
    pub ui_geometries: Vec<GeometryRenderData<'a>>,
}

pub struct RendererBackend<'a> {
    frame_number: u64,
    initialize:
        fn(application_name: &str, window: &Window) -> std::result::Result<(), VulkanBackendError>,
    shutdown: fn() -> std::result::Result<(), VulkanBackendError>,
    resized: fn(width: i32, height: i32) -> std::result::Result<(), VulkanBackendError>,
    begin_frame: fn(delta_time: f32) -> std::result::Result<bool, VulkanBackendError>,
    update_global_world_state: fn(
        projection: Matrix4,
        view: Matrix4,
        view_position: Vec3,
        ambient_colour: Vec4,
        mode: i32,
    ) -> std::result::Result<(), VulkanBackendError>,
    update_global_ui_state: fn(
        projection: Matrix4,
        view: Matrix4,
        mode: i32,
    ) -> std::result::Result<(), VulkanBackendError>,
    begin_renderpass:
        fn(renderpass_type: BuiltInRenderpass) -> std::result::Result<(), VulkanBackendError>,
    end_renderpass:
        fn(renderpass_type: BuiltInRenderpass) -> std::result::Result<(), VulkanBackendError>,
    draw_geometry: fn(data: &mut GeometryRenderData) -> std::result::Result<(), VulkanBackendError>,
    create_texture:
        fn(pixels: &[u8], texture: &mut Texture) -> std::result::Result<(), VulkanBackendError>,
    destroy_texture: fn(texture: &Texture) -> std::result::Result<(), VulkanBackendError>,
    create_material:
        fn(material: &'_ mut Material<'a>) -> std::result::Result<(), VulkanBackendError>,
    destroy_material: fn(material: &Material<'a>) -> std::result::Result<(), VulkanBackendError>,
    // hard to figure out generics
    // create_geometry: fn(
    //     geometry: &'_ mut Geometry<'a>,
    //     vertices: &[Vector3D],
    //     indices: &[u32],
    // ) -> std::result::Result<(), VulkanBackendError>,
    destroy_geometry: fn(geometry: &Geometry<'a>) -> std::result::Result<(), VulkanBackendError>,
    end_frame: fn(delta_time: f32) -> std::result::Result<(), VulkanBackendError>,

    projection: Matrix4,
    view: Matrix4,
    ui_projection: Matrix4,
    ui_view: Matrix4,
    far_clip: f32,
    near_clip: f32,
}

static mut RENDERER_BACKEND: Option<RendererBackend> = None;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("frontend renderer error: already initialized {} {}", file, line)]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("frontend renderer error: already shutdown {} {}", file, line)]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("frontend renderer error: not initialized {} {}", file, line)]
    NotInitialized { file: &'static str, line: u32 },
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
}
pub struct Renderer;

impl<'a> Renderer {
    pub fn initialize(app_name: &str, window: &Window) -> Result<()> {
        unsafe {
            if let Some(ref _state) = RENDERER_BACKEND {
                return Err(RendererError::AlreadyInitialized {
                    file: file!(),
                    line: line!(),
                });
            } else {
                Self::create_renderer_backend(&RendererBackendType::Vulkan)?;
            }

            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.initialize)(app_name, window).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })?;
                Ok(())
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn shutdown() -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                let _ = Self::destroy_renderer_backend();
                RENDERER_BACKEND = None;
                return Ok(());
            } else {
                return Err(RendererError::AlreadyShutdown {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn create_texture(pixels: &[u8], texture: &mut Texture) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.create_texture)(pixels, texture).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn destroy_texture(texture: &Texture) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_texture)(&texture).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn create_material<'b: 'static>(material: &'a mut Material<'b>) -> Result<()> {
        unsafe {
            if let Some(ref state) = RENDERER_BACKEND {
                (state.create_material)(material).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn destroy_material<'b: 'static>(material: &'a Material<'b>) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_material)(material).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn create_geometry<'b: 'static, T: Clone, U: Clone>(
        geometry: &'a mut Geometry<'b>,
        vertices: &[T],
        indicies: &[U],
    ) -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                VulkanContext::create_geometry(geometry, vertices, indicies).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn destroy_geometry<'b: 'static>(geometry: &'a Geometry<'b>) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_geometry)(geometry).map_err(|e| {
                    RendererError::BackendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn draw_frame(packet: &mut RendererPacket) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        if !(state.begin_frame)(packet.delta_time).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })? {
            return Ok(());
        }

        (state.begin_renderpass)(BuiltInRenderpass::World).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        (state.update_global_world_state)(
            state.projection,
            state.view,
            Vec3::new_zeroes(),
            Vec4::new_zeroes(),
            0,
        )
        .map_err(|e| RendererError::BackendRendererError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        for geo in packet.geometries.iter_mut() {
            (state.draw_geometry)(geo).map_err(|e| {
                RendererError::BackendRendererError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?;
        }

        (state.end_renderpass)(BuiltInRenderpass::World).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        (state.begin_renderpass)(BuiltInRenderpass::UI).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        (state.update_global_ui_state)(state.ui_projection, state.ui_view, 0).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        for geo in packet.ui_geometries.iter_mut() {
            (state.draw_geometry)(geo).map_err(|e| {
                RendererError::BackendRendererError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?;
        }

        (state.end_renderpass)(BuiltInRenderpass::UI).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        (state.end_frame)(packet.delta_time).map_err(|e| {
            RendererError::BackendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        state.frame_number += 1;
        Ok(())
    }

    pub fn on_resize(width: i32, height: i32) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        state.projection = Matrix4::perspective(
            45.0,
            width as f32 / height as f32,
            state.near_clip,
            state.far_clip,
        );
        state.ui_projection =
            Matrix4::orthographic(0.0, width as f32, height as f32, 0.0, -100.0, 100.0);
        (state.resized)(width, height).map_err(|e| RendererError::BackendRendererError {
            source: e,
            file: file!(),
            line: line!(),
        })
    }

    pub fn set_view(view: Matrix4) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.view = view;
                Ok(())
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    fn create_renderer_backend(_type_: &RendererBackendType) -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                return Err(RendererError::AlreadyInitialized {
                    file: file!(),
                    line: line!(),
                });
            } else {
                RENDERER_BACKEND = Some(RendererBackend {
                    frame_number: 0,
                    initialize: VulkanContext::initialize,
                    shutdown: VulkanContext::shutdown,
                    resized: VulkanContext::on_resize,
                    begin_frame: VulkanContext::begin_frame,
                    end_frame: VulkanContext::end_frame,
                    update_global_world_state: VulkanContext::update_global_world_state,
                    update_global_ui_state: VulkanContext::update_global_ui_state,
                    begin_renderpass: VulkanContext::begin_renderpass,
                    end_renderpass: VulkanContext::end_renderpass,
                    draw_geometry: VulkanContext::draw_geometry,
                    projection: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 1000.0),
                    view: Matrix4::translation(&Vec3::new(0.0, 0.0, -30.0)),
                    ui_projection: Matrix4::orthographic(0.0, 1280.0, 720.0, 0.0, -100.0, 100.0),
                    ui_view: Matrix4::identity(),
                    far_clip: 1000.0,
                    near_clip: 0.1,
                    create_texture: VulkanContext::create_texture,
                    destroy_texture: VulkanContext::destroy_texture,
                    create_material: VulkanContext::create_material,
                    destroy_material: VulkanContext::destroy_material,
                    //create_geometry: VulkanContext::create_geometry,
                    destroy_geometry: VulkanContext::destroy_geometry,
                })
            }
        }
        Ok(())
    }

    fn destroy_renderer_backend() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                Ok(
                    (state.shutdown)().map_err(|e| {
                        RendererError::BackendRendererError {
                            source: e,
                            file: file!(),
                            line: line!(),
                        }
                    })?,
                )
            } else {
                return Err(RendererError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }
}
