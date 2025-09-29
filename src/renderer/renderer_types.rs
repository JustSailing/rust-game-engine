use crate::application::{
    basic::{
        event::{EventCodes, EventCtx, EventState},
        math::consts::INVALID_ID,
    },
    renderer::vulkan::vulkan_backend::VulkanContext,
};

use super::resources::resource_types::Texture;

use std::{cell::RefCell, fmt, ptr, rc::Rc};

use crate::application::basic::{
    math::{consts::deg_to_rad, matrix4::Matrix4, vec3::Vec3, vec4::Quat, vec4::Vec4},
    window::Window,
};

use image;

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

pub struct GeometryRenderData {
    pub object_id: usize,
    pub model: Matrix4,
    pub textures: [Rc<RefCell<Option<Texture>>>; 1],
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
    update_object: fn(data: &mut GeometryRenderData) -> Result<()>,
    create_texture: fn(
        name: &str,
        auto_realease: bool,
        width: u32,
        height: u32,
        channel_count: u32,
        pixels: &[u8],
        has_transparency: bool,
    ) -> Result<Texture>,
    set_default_diffuse: fn(texture: &Texture) -> Result<()>,
    destroy_texture: fn(texture: &Texture) -> Result<()>,
    end_frame: fn(delta_time: f32) -> Result<()>,

    projection: Matrix4,
    view: Matrix4,
    far_clip: f32,
    near_clip: f32,
    default_diffuse: Rc<RefCell<Option<Texture>>>,
    test_diffuse: Rc<RefCell<Option<Texture>>>,
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
            Error::AlreadyInitialized => write!(
                f,
                "FrontendRenderer Already Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::AlreadyShutdown => write!(
                f,
                "FrontendRenderer Already Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::NotInitialized => write!(
                f,
                "FrontendRenderer Already Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::OperationFailed(e) => write!(
                f,
                "FrontendRenderer Operation Failed: {e}. {}  {}",
                file!(),
                line!()
            ),
        }
    }
}

impl std::error::Error for Error {}

pub struct FrontendRenderer;

impl FrontendRenderer {
    pub fn initialize(app_name: &str, window: &Window) -> Result<()> {
        unsafe {
            if let Some(ref _state) = RENDERER_BACKEND {
                return Err(Error::AlreadyInitialized.into());
            } else {
                FrontendRenderer::create_renderer_backend(&RendererBackendType::Vulkan)?;
            }

            EventState::register_event(
                EventCodes::Debug0 as usize,
                ptr::null(),
                Self::on_event_debug,
            )?;

            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.initialize)(app_name, window)?;
                const TEX_DIMENSION: u8 = 255;
                const CHANNELS: u8 = 4;
                const PIXEL_COUNT: usize = TEX_DIMENSION as usize * TEX_DIMENSION as usize;
                let mut pixels = [255u8; PIXEL_COUNT * CHANNELS as usize];
                let width = CHANNELS;
                let height: usize = pixels.len() / CHANNELS as usize;
                for row in (0..height).step_by(4) {
                    for col in (0..width as usize).step_by(4) {
                        let index = (row * width as usize) + col;

                        pixels[index + 0] = 0;
                        pixels[index + 1] = 0;
                    }
                }

                let texture = Self::create_texture(
                    "default",
                    false,
                    TEX_DIMENSION as u32,
                    TEX_DIMENSION as u32,
                    CHANNELS as u32,
                    &pixels,
                    false,
                )?;
                //texture.generation = INVALID_ID as u32;
                state.default_diffuse = Rc::new(RefCell::new(Some(texture)));

                //state.test_diffuse = Some(texture);
                (state.set_default_diffuse)(&state.default_diffuse.borrow().as_ref().unwrap())?;
                Ok(())
            } else {
                return Err(Error::NotInitialized.into());
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

    pub fn create_texture(
        name: &str,
        auto_realease: bool,
        width: u32,
        height: u32,
        channel_count: u32,
        pixels: &[u8],
        has_transparency: bool,
    ) -> Result<Texture> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                (state.create_texture)(
                    name,
                    auto_realease,
                    width,
                    height,
                    channel_count,
                    pixels,
                    has_transparency,
                )
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }

    fn load_texture(name: &str) -> Result<Texture> {
        let file_path = format!("assets/textures/{}.{}", name, "jpg");
        let data = image::open(file_path)?.to_rgba8();
        let width = data.width();
        let height = data.height();
        let v = data.into_raw();
        let channel_count: u32 = 4;
        let total_size = width * height * channel_count;
        let mut transparancy = false;
        for i in (3..total_size as usize).step_by(channel_count as usize) {
            if v[i] < 255 {
                transparancy = true;
                break;
            }
        }
        let mut texture = Self::create_texture(
            name,
            true,
            width as u32,
            height as u32,
            channel_count,
            v.as_slice(),
            transparancy,
        )?;
        texture.generation = INVALID_ID as u32;

        Ok(texture)
    }

    pub fn on_event_debug(
        _code: usize,
        _sender: *const std::ffi::c_void,
        _listener: *const std::ffi::c_void,
        _ctx: &EventCtx,
    ) -> bool {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return false;
            }
        };
        let names = ["brick-wall", "door", "stone-wall", "tile"];
        static mut CHOICE: usize = 3;
        unsafe {
            CHOICE += 1;
            CHOICE %= 4;
        }
        let temp = state.default_diffuse.borrow_mut().unwrap();
        state
            .default_diffuse
            .replace(match Self::load_texture(names[unsafe { CHOICE }]) {
                Ok(t) => Some(t),
                Err(_) => return false,
            });
        let _ = Self::destroy_texture(&temp);
        true
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

    pub fn draw_frame(packet: &RendererPacket) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                state
            } else {
                return Err(Error::NotInitialized.into());
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
        let mut data = GeometryRenderData {
            object_id: 0,
            model,
            textures: [Rc::new(RefCell::new(None))],
        };
        data.textures[0] = Rc::clone(&state.default_diffuse);
        (state.update_object)(&mut data)?;

        FrontendRenderer::end_frame(packet.delta_time)?;
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
                    update_object: VulkanContext::update_object,
                    projection: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 1000.0),
                    view: Matrix4::translation(&Vec3::new(0.0, 0.0, -30.0)),
                    far_clip: 1000.0,
                    near_clip: 0.1,
                    create_texture: VulkanContext::create_texture,
                    destroy_texture: VulkanContext::destroy_texture,
                    default_diffuse: Rc::new(RefCell::new(std::mem::zeroed())),
                    test_diffuse: Rc::new(RefCell::new(std::mem::zeroed())),
                    set_default_diffuse: VulkanContext::set_default_diffuse,
                })
            }
        }
        Ok(())
    }

    fn destroy_renderer_backend() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = RENDERER_BACKEND {
                Self::destroy_texture(state.default_diffuse.borrow().as_ref().unwrap())?;
                Ok((state.shutdown)()?)
            } else {
                return Err(Error::NotInitialized.into());
            }
        }
    }
}
