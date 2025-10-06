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
    renderer::vulkan::vulkan_backend::VulkanContext,
    resources::resource_types::{Geometry, Material, Texture},
};

use std::{cell::RefCell, fmt, rc::Rc};

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

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;
pub struct RendererBackend<'a> {
    frame_number: u64,
    initialize: fn(application_name: &str, window: &Window) -> Result<()>,
    shutdown: fn() -> Result<()>,
    resized: fn(width: i32, height: i32) -> Result<()>,
    begin_frame: fn(delta_time: f32) -> Result<bool>,
    update_global_state: fn(
        projection: Matrix4,
        view: Matrix4,
        view_position: Vec3,
        ambient_colour: Vec4,
        mode: i32,
    ) -> Result<()>,
    draw_geometry: fn(data: &mut GeometryRenderData) -> Result<()>,
    create_texture: fn(pixels: &[u8], texture: &mut Texture) -> Result<()>,
    destroy_texture: fn(texture: &Texture) -> Result<()>,
    create_material: fn(material: &'_ mut Material<'a>) -> Result<()>,
    destroy_material: fn(material: &Material<'a>) -> Result<()>,
    create_geometry:
        fn(geometry: &'_ mut Geometry<'a>, vertices: &[Vector3D], indices: &[u32]) -> Result<()>,
    destroy_geometry: fn(geometry: &Geometry<'a>) -> Result<()>,
    end_frame: fn(delta_time: f32) -> Result<()>,

    projection: Matrix4,
    view: Matrix4,
    far_clip: f32,
    near_clip: f32,
}

static mut RENDERER_BACKEND: Option<RendererBackend> = None;

#[derive(Debug)]
pub enum Error {
    AlreadyInitialized,
    AlreadyShutdown,
    NotInitialized,
    OperationFailed(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyInitialized => {
                write!(f, "Renderer Already Initialized {}  {}", file!(), line!())
            }
            Error::AlreadyShutdown => {
                write!(f, "Renderer Already Initialized {}  {}", file!(), line!())
            }
            Error::NotInitialized => {
                write!(f, "Renderer Already Initialized {}  {}", file!(), line!())
            }
            Error::OperationFailed(e) => write!(
                f,
                "Renderer Operation Failed: {e}. {}  {}",
                file!(),
                line!()
            ),
        }
    }
}

impl std::error::Error for Error {}

pub struct Renderer;

impl<'a> Renderer {
    pub fn initialize(app_name: &str, window: &Window) -> Result<()> {
        unsafe {
            if let Some(ref _state) = RENDERER_BACKEND {
                return Err(Error::AlreadyInitialized.into());
            } else {
                Self::create_renderer_backend(&RendererBackendType::Vulkan)?;
            }

            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.initialize)(app_name, window)?;
                Ok(())
            } else {
                return Err(Error::NotInitialized.into());
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
                return Err(Error::AlreadyShutdown.into());
            }
        }
    }

    pub fn begin_frame(delta: f32) -> Result<bool> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.begin_frame)(delta)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn create_texture(pixels: &[u8], texture: &mut Texture) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.create_texture)(pixels, texture)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn destroy_texture(texture: &Texture) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_texture)(&texture)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn create_material<'b: 'static>(material: &'a mut Material<'b>) -> Result<()> {
        unsafe {
            if let Some(ref state) = RENDERER_BACKEND {
                (state.create_material)(material)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn destroy_material<'b: 'static>(material: &'a Material<'b>) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_material)(material)
            } else {
                return Err(Error::NotInitialized.into());
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
                (state.create_geometry)(geometry, vertices, indicies)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn destroy_geometry<'b: 'static>(geometry: &'a Geometry<'b>) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.destroy_geometry)(geometry)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn end_frame(delta: f32) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.frame_number += 1;
                (state.end_frame)(delta)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    pub fn draw_frame(packet: &mut RendererPacket) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(Error::NotInitialized.into());
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
                return Err(Error::NotInitialized.into());
            }
        };
        state.projection = Matrix4::perspective(
            45.0,
            width as f32 / height as f32,
            state.near_clip,
            state.far_clip,
        );
        (state.resized)(width, height)
    }

    pub fn set_view(view: Matrix4) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.view = view;
                Ok(())
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    fn create_renderer_backend(_type_: &RendererBackendType) -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                return Err(Error::AlreadyInitialized.into());
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
                return Err(Error::NotInitialized.into());
            }
        }
    }
}
