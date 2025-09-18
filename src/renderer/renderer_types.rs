#[path = "./vulkan/mod.rs"]
mod vulkan;

use vulkan::vulkan_backend::{VulkanContext, VulkanError};

use crate::application::basic::window::Window;
pub enum RendererBackendType {
    Vulkan,
    OpenGL,
    DirectX,
}

pub struct RendererPacket {
    pub delta_time: f32,
}

pub struct RendererBackend {
    frame_number: u64,
    initialize: fn(application_name: &str, window: &Window) -> Result<(), VulkanError>,
    shutdown: fn() -> Result<(), VulkanError>,
    resized: fn(width: i32, height: i32) -> Result<(), VulkanError>,
    begin_frame: fn(delta_time: f32) -> Result<bool, VulkanError>,
    end_frame: fn(delta_time: f32) -> Result<(), VulkanError>,
}

static mut RENDERER_BACKEND: Option<RendererBackend> = None;

pub enum FrontendRendererError {
    AlreadyInitialized,
    AlreadyShutdown,
    NotInitialized,
    OperationFailed(&'static str),
}

impl From<VulkanError> for FrontendRendererError {
    fn from(value: VulkanError) -> Self {
        match value {
            VulkanError::OperationFailed(e) => FrontendRendererError::OperationFailed(e),
        }
    }
}

pub struct FrontendRenderer;

impl FrontendRenderer {
    pub fn initialize(app_name: &str, window: &Window) -> Result<(), FrontendRendererError> {
        unsafe {
            if let Some(ref _state) = RENDERER_BACKEND {
                return Err(FrontendRendererError::AlreadyInitialized);
            } else {
                let _ =
                    match FrontendRenderer::create_renderer_backend(&RendererBackendType::Vulkan) {
                        Ok(_) => (),
                        Err(e) => return Err(e),
                    };
            }
            if let Some(ref state) = RENDERER_BACKEND {
                match (state.initialize)(app_name, window) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(FrontendRendererError::from(e)),
                }
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn shutdown() -> Result<(), FrontendRendererError> {
        unsafe {
            if let Some(ref mut _state) = RENDERER_BACKEND {
                let _ = FrontendRenderer::destroy_renderer_backend();
                RENDERER_BACKEND = None;
                return Ok(());
            } else {
                return Err(FrontendRendererError::AlreadyShutdown);
            }
        }
    }

    pub fn begin_frame(delta: f32) -> Result<bool, FrontendRendererError> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                match (state.begin_frame)(delta) {
                    Ok(b) => return Ok(b),
                    Err(e) => Err(FrontendRendererError::from(e)),
                }
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn end_frame(delta: f32) -> Result<(), FrontendRendererError> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state.frame_number += 1;
                match (state.end_frame)(delta) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(FrontendRendererError::from(e)),
                }
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }

    pub fn draw_frame(packet: &RendererPacket) -> Result<(), FrontendRendererError> {
        match FrontendRenderer::begin_frame(packet.delta_time) {
            Ok(b) => match b {
                false => return Ok(()),
                true => match FrontendRenderer::end_frame(packet.delta_time) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                },
            },

            Err(e) => return Err(e),
        }
        Ok(())
    }

    pub fn on_resize(width: i32, height: i32) -> Result<(), FrontendRendererError> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                match (state.resized)(width, height) {
                    Ok(_) => (),
                    Err(_) => {
                        return Err(FrontendRendererError::OperationFailed(
                            "failed to resize window",
                        ));
                    }
                }
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
        Ok(())
    }

    fn create_renderer_backend(type_: &RendererBackendType) -> Result<(), FrontendRendererError> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                return Err(FrontendRendererError::AlreadyInitialized);
            } else {
                RENDERER_BACKEND = Some(RendererBackend {
                    frame_number: 0,
                    initialize: VulkanContext::initialize,
                    shutdown: VulkanContext::shutdown,
                    resized: VulkanContext::on_resize,
                    begin_frame: VulkanContext::begin_frame,
                    end_frame: VulkanContext::end_frame,
                })
            }
        }
        Ok(())
    }

    fn destroy_renderer_backend() -> Result<(), FrontendRendererError> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                match (state.shutdown)() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(FrontendRendererError::from(e)),
                }
            } else {
                return Err(FrontendRendererError::NotInitialized);
            }
        }
    }
}
