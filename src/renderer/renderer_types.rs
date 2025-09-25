#[path = "./vulkan/mod.rs"]
mod vulkan;

use std::{error::Error, fmt};
use vulkan::vulkan_backend::VulkanContext;

use crate::application::basic::{
    math::{consts::deg_to_rad, matrix4::Matrix4, vec3::Vec3, vec4::Quat, vec4::Vec4},
    window::Window,
};
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

pub struct RendererPacket {
    pub delta_time: f32,
}

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;
pub struct RendererBackend {
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
    update_object: fn(model: Matrix4) -> Result<()>,
    end_frame: fn(delta_time: f32) -> Result<()>,
    projection: Matrix4,
    view: Matrix4,
    far_clip: f32,
    near_clip: f32,
}

static mut RENDERER_BACKEND: Option<RendererBackend> = None;

#[derive(Debug)]
pub enum FrontendRendererError {
    AlreadyInitialized,
    AlreadyShutdown,
    NotInitialized,
    OperationFailed(&'static str),
}

impl fmt::Display for FrontendRendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontendRendererError::AlreadyInitialized => write!(
                f,
                "FrontendRenderer Already Initialized {}  {}",
                file!(),
                line!()
            ),
            FrontendRendererError::AlreadyShutdown => write!(
                f,
                "FrontendRenderer Already Initialized {}  {}",
                file!(),
                line!()
            ),
            FrontendRendererError::NotInitialized => write!(
                f,
                "FrontendRenderer Already Initialized {}  {}",
                file!(),
                line!()
            ),
            FrontendRendererError::OperationFailed(e) => write!(
                f,
                "FrontendRenderer Operation Failed: {e}. {}  {}",
                file!(),
                line!()
            ),
        }
    }
}

impl Error for FrontendRendererError {}

pub struct FrontendRenderer;

impl FrontendRenderer {
    pub fn initialize(app_name: &str, window: &Window) -> Result<()> {
        unsafe {
            if let Some(ref _state) = RENDERER_BACKEND {
                return Err(FrontendRendererError::AlreadyInitialized.into());
            } else {
                FrontendRenderer::create_renderer_backend(&RendererBackendType::Vulkan)?;
            }
            if let Some(ref state) = RENDERER_BACKEND {
                (state.initialize)(app_name, window)
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
            }
        }
    }

    pub fn shutdown() -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                let _ = FrontendRenderer::destroy_renderer_backend();
                RENDERER_BACKEND = None;
                return Ok(());
            } else {
                return Err(FrontendRendererError::AlreadyShutdown.into());
            }
        }
    }

    pub fn begin_frame(delta: f32) -> Result<bool> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.begin_frame)(delta)
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
            }
        }
    }

    pub fn end_frame(delta: f32) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.frame_number += 1;
                (state.end_frame)(delta)
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
            }
        }
    }

    pub fn draw_frame(packet: &RendererPacket) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
            }
        };

        if !FrontendRenderer::begin_frame(packet.delta_time)? {
            return Ok(());
        }

        (state.update_global_state)(
            state.projection,
            state.view,
            Vec3::new_zeroes(),
            Vec4::new_zeroes(),
            0,
        )?;

        static mut ANGLE: f32 = 0.01;
        unsafe { ANGLE += 0.01 }

        let rotation = unsafe { Quat::from_axis_angle(Vec3::new_forward(), ANGLE, false) };
        let model = Quat::to_rotation_matrix(rotation, Vec3::new_zeroes());
        // let model = Matrix4::identity();
        (state.update_object)(model)?;

        FrontendRenderer::end_frame(packet.delta_time)?;
        Ok(())
    }

    pub fn on_resize(width: i32, height: i32) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(FrontendRendererError::NotInitialized.into());
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
                    update_object: VulkanContext::update_object,
                    projection: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 1000.0),
                    view: Matrix4::translation(&Vec3::new(0.0, 0.0, -30.0)),
                    far_clip: 1000.0,
                    near_clip: 0.1,
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
