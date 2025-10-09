use crate::application::{
    basic::{
        math::{
            consts::deg_to_rad,
            matrix4::Matrix4,
            vec3::{Vec3, Vector3D},
            vec4::Vec4,
        },
        window::Window,
    },
    renderer::vulkan::vulkan_backend::{VulkanBackendError, VulkanContext},
    resources::resource_types::{Geometry, Material, Texture},
};

use std::{cell::RefCell, rc::Rc};

use thiserror::Error;

type Result<T> = std::result::Result<T, FrontendRendererError>;
pub enum RendererBackendType {
    Vulkan,
    OpenGL,
    DirectX,
}

#[derive(Copy, Clone)]
#[repr(C, align(16))] // both C and align are needed or some funky stuff happens
pub struct GlobalUniformObj {
    pub projection: Matrix4,
    pub view: Matrix4,
    pub padding: [Matrix4; 2], // for NVidia cards
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct UniformObject {
    pub diffuse_color: Vec4,
    pub padding: [Vec4; 3],
}

pub struct GeometryRenderData<'a> {
    pub model: Matrix4,
    pub geometry: Rc<RefCell<Option<&'a mut Geometry<'a>>>>,
}

pub struct RendererPacket<'a> {
    pub delta_time: f32,
    pub geometries: Vec<GeometryRenderData<'a>>,
}

pub struct RendererBackend<'a> {
    frame_number: u64,
    initialize:
        fn(application_name: &str, window: &Window) -> std::result::Result<(), VulkanBackendError>,
    shutdown: fn() -> std::result::Result<(), VulkanBackendError>,
    resized: fn(width: i32, height: i32) -> std::result::Result<(), VulkanBackendError>,
    begin_frame: fn(delta_time: f32) -> std::result::Result<bool, VulkanBackendError>,
    update_global_state: fn(
        projection: Matrix4,
        view: Matrix4,
        view_position: Vec3,
        ambient_colour: Vec4,
        mode: i32,
    ) -> std::result::Result<(), VulkanBackendError>,
    draw_geometry: fn(data: &mut GeometryRenderData) -> std::result::Result<(), VulkanBackendError>,
    create_texture:
        fn(pixels: &[u8], texture: &mut Texture) -> std::result::Result<(), VulkanBackendError>,
    destroy_texture: fn(texture: &Texture) -> std::result::Result<(), VulkanBackendError>,
    create_material:
        fn(material: &'_ mut Material<'a>) -> std::result::Result<(), VulkanBackendError>,
    destroy_material: fn(material: &Material<'a>) -> std::result::Result<(), VulkanBackendError>,
    create_geometry: fn(
        geometry: &'_ mut Geometry<'a>,
        vertices: &[Vector3D],
        indices: &[u32],
    ) -> std::result::Result<(), VulkanBackendError>,
    destroy_geometry: fn(geometry: &Geometry<'a>) -> std::result::Result<(), VulkanBackendError>,
    end_frame: fn(delta_time: f32) -> std::result::Result<(), VulkanBackendError>,

    projection: Matrix4,
    view: Matrix4,
    far_clip: f32,
    near_clip: f32,
}

static mut RENDERER_BACKEND: Option<RendererBackend> = None;

#[derive(Error, Debug)]
pub enum FrontendRendererError {
    #[error("frontend renderer error: already initialized {} {}", file!(), line!())]
    AlreadyInitialized,
    #[error("frontend renderer error: already shutdown {} {}", file!(), line!())]
    AlreadyShutdown,
    #[error("frontend renderer error: not initialized {} {}", file!(), line!())]
    NotInitialized,
    #[error("{source}\nfrontend renderer error: backend renderer error {} {}", file!(), line!())]
    BackendRendererError {
        #[from]
        source: VulkanBackendError,
    },
}

pub struct Renderer;

impl<'a> Renderer {
    pub fn initialize(app_name: &str, window: &Window) -> Result<()> {
        unsafe {
            if let Some(ref _state) = RENDERER_BACKEND {
                return Err(FrontendRendererError::AlreadyInitialized);
            } else {
                Self::create_renderer_backend(&RendererBackendType::Vulkan)?;
            }

            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.initialize)(app_name, window)?;
                Ok(())
            } else {
                return Err(FrontendRendererError::NotInitialized);
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
                return Err(FrontendRendererError::AlreadyShutdown);
            }
        }
    }

    pub fn begin_frame(delta: f32) -> Result<bool> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.begin_frame)(delta).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn create_texture(pixels: &[u8], texture: &mut Texture) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.create_texture)(pixels, texture).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn destroy_texture(texture: &Texture) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_texture)(&texture).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn create_material<'b: 'static>(material: &'a mut Material<'b>) -> Result<()> {
        unsafe {
            if let Some(ref state) = RENDERER_BACKEND {
                (state.create_material)(material).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn destroy_material<'b: 'static>(material: &'a Material<'b>) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_material)(material).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn create_geometry<'b: 'static>(
        geometry: &'a mut Geometry<'b>,
        vertices: &[Vector3D],
        indicies: &[u32],
    ) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.create_geometry)(geometry, vertices, indicies).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn destroy_geometry<'b: 'static>(geometry: &'a Geometry<'b>) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_geometry)(geometry).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn end_frame(delta: f32) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.frame_number += 1;
                (state.end_frame)(delta).map_err(Into::into)
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn draw_frame(packet: &mut RendererPacket) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        };

        if !Self::begin_frame(packet.delta_time)? {
            return Ok(());
        }

        (state.update_global_state)(
            state.projection,
            state.view,
            Vec3::new_zeroes(),
            Vec4::new_zeroes(),
            0,
        )?;

        for geo in packet.geometries.iter_mut() {
            (state.draw_geometry)(geo)?;
        }

        Renderer::end_frame(packet.delta_time)?;
        Ok(())
    }

    pub fn on_resize(width: i32, height: i32) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        };
        state.projection = Matrix4::perspective(
            45.0,
            width as f32 / height as f32,
            state.near_clip,
            state.far_clip,
        );
        (state.resized)(width, height).map_err(Into::into)
    }

    pub fn set_view(view: Matrix4) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.view = view;
                Ok(())
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
            }
        }
    }

    fn create_renderer_backend(_type_: &RendererBackendType) -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                return Err(FrontendRendererError::AlreadyInitialized.into());
            } else {
                RENDERER_BACKEND = Some(RendererBackend {
                    frame_number: 0,
                    initialize: VulkanContext::initialize,
                    shutdown: VulkanContext::shutdown,
                    resized: VulkanContext::on_resize,
                    begin_frame: VulkanContext::begin_frame,
                    end_frame: VulkanContext::end_frame,
                    update_global_state: VulkanContext::update_global_state,
                    draw_geometry: VulkanContext::draw_geometry,
                    projection: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 1000.0),
                    view: Matrix4::translation(&Vec3::new(0.0, 0.0, -30.0)),
                    far_clip: 1000.0,
                    near_clip: 0.1,
                    create_texture: VulkanContext::create_texture,
                    destroy_texture: VulkanContext::destroy_texture,
                    create_material: VulkanContext::create_material,
                    destroy_material: VulkanContext::destroy_material,
                    create_geometry: VulkanContext::create_geometry,
                    destroy_geometry: VulkanContext::destroy_geometry,
                })
            }
        }
        Ok(())
    }

    fn destroy_renderer_backend() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                Ok((state.shutdown)()?)
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
            }
        }
    }
}
