#[path = "./basic/mod.rs"]
pub mod basic;
#[path = "./renderer/mod.rs"]
pub mod renderer;

#[path = "./resources/mod.rs"]
pub mod resources;

#[path = "./systems/mod.rs"]
pub mod systems;

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};
use std::{ffi::c_void, ptr};
use thiserror::Error;

use crate::Game;
use basic::event::{EventCodes, EventCtx, EventState, EventSysError};
use basic::input::{InputState, InputSysError};
use basic::math::matrix4::Matrix4;
use basic::window::{Window, WindowError};
use renderer::renderer_types::{
    FrontendRendererError, GeometryRenderData, Renderer, RendererPacket,
};
use resources::resource_types::Geometry;
use systems::geometry_system::{GeometrySysConfig, GeometrySysError, GeometrySystem};
use systems::material_system::{MaterialSysConfig, MaterialSysError, MaterialSystem};
use systems::texture_system::{TextureSysConfig, TextureSysError, TextureSystem};

#[derive(Clone, Copy)]
pub struct AppConfig {
    pub start_pos_x: i32,
    pub start_pos_y: i32,
    pub start_width: i32,
    pub start_height: i32,
    pub name: &'static str,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("app error: could not create window {} {}", file!(), line!())]
    CouldNotCreateWindow,
    #[error("app error: already initialized {} {}", file!(), line!())]
    AlreadyInitialized,
    #[error("app error: already shutdown {} {}", file!(), line!())]
    AlreadyShutdown,
    #[error("appp error: not initialized {} {}", file!(), line!())]
    NotInitialized,
    #[error("app error: could not initialize game {} {}", file!(), line!())]
    CouldNotInitializeGame,
    #[error("app error:  could not update game {} {}", file!(), line!())]
    CouldNotUpdateGame,
    #[error("app error:  could not render game {} {}", file!(), line!())]
    CouldNotRenderGame,
    #[error("app error:  error from event system {source} {} {}", file!(), line!())]
    EventSysError {
        #[from]
        source: EventSysError,
    },
    #[error("app error:  error from input system {source} {} {}", file!(), line!())]
    InputSysError {
        #[from]
        source: InputSysError,
    },
    #[error("app error:  error from window {source} {} {}", file!(), line!())]
    WindowError {
        #[from]
        source: WindowError,
    },
    #[error("app error:  error from renderer frontend {source} {} {}", file!(), line!())]
    FrontendRendererError {
        #[from]
        source: FrontendRendererError,
    },
    #[error("app error:  error from material system {source} {} {}", file!(), line!())]
    MaterialSysError {
        #[from]
        source: MaterialSysError,
    },
    #[error("app error:  error from texture system {source} {} {}", file!(), line!())]
    TextureSysError {
        #[from]
        source: TextureSysError,
    },
    #[error("app error:  error from geometry system {source} {} {}", file!(), line!())]
    GeometrySysError {
        #[from]
        source: GeometrySysError,
    },
}

type Result<T> = std::result::Result<T, AppError>;

pub struct ApplicationState<'a> {
    game: Game,
    is_running: bool,
    is_suspended: bool,
    window: Window,
    pos_x: i32,
    pos_y: i32,
    width: i32,
    height: i32,
    test_geometry: Rc<RefCell<Option<&'a mut Geometry<'a>>>>,
}

static mut APP_STATE: Option<ApplicationState> = None;

impl<'a: 'static> ApplicationState<'a> {
    pub fn create(game: &mut Game) -> Result<()> {
        unsafe {
            if let Some(ref _a) = APP_STATE {
                return Err(AppError::AlreadyInitialized);
            }
        }
        let app_config = game.config;
        let window = Window::create(
            app_config.start_pos_x,
            app_config.start_pos_y,
            app_config.start_width,
            app_config.start_height,
        )?;

        window.set_title(app_config.name);
        window.show();

        InputState::initialize()?;

        EventState::initialize()?;

        EventState::register_event(
            EventCodes::ApplicationQuit as usize,
            ptr::null(),
            application_on_event,
        )?;

        EventState::register_event(
            EventCodes::WindowResized as usize,
            ptr::null(),
            application_on_resize,
        )?;

        EventState::register_event(EventCodes::Debug0 as usize, ptr::null(), on_event_debug)?;

        Renderer::initialize(app_config.name, &window)?;

        let texture_sys_config: TextureSysConfig = TextureSysConfig { max_count: 100 };
        TextureSystem::initialize(texture_sys_config)?;

        let material_sys_config: MaterialSysConfig = MaterialSysConfig { max_count: 100 };
        MaterialSystem::initialize(material_sys_config)?;

        let geometry_sys_config: GeometrySysConfig = GeometrySysConfig { max_count: 100 };
        GeometrySystem::initialize(geometry_sys_config)?;

        if !(game.initialize)(game) {
            return Err(AppError::CouldNotInitializeGame);
        }

        unsafe {
            APP_STATE = Some(ApplicationState {
                game: *game,
                is_running: false,
                is_suspended: false,
                window: window,
                pos_x: app_config.start_pos_x,
                pos_y: app_config.start_pos_y,
                width: app_config.start_width,
                height: app_config.start_height,
                test_geometry: Rc::new(RefCell::new(Some(GeometrySystem::get_default_geometry()?))),
            });
        }

        Ok(())
    }

    pub fn run() -> Result<()> {
        let app_state = unsafe {
            if let Some(ref mut app) = APP_STATE {
                app
            } else {
                return Err(AppError::NotInitialized);
            }
        };

        app_state.is_running = true;
        app_state.is_suspended = false;
        const FPS: f32 = 60.0;
        let frame_duration: Duration = Duration::from_secs_f32(1.0 / FPS);
        let mut last_frame_time = Instant::now();
        loop {
            if app_state.window.get_event()? {
                let current_time = Instant::now();
                let delta = current_time.duration_since(last_frame_time).as_secs_f32() / 60.0;
                InputState::update(delta)?;
                if !(app_state.game.update)(&mut app_state.game, delta) {
                    return Err(AppError::CouldNotUpdateGame);
                }

                if !(app_state.game.render)(&mut app_state.game, delta) {
                    return Err(AppError::CouldNotRenderGame);
                }
                let geo = Rc::clone(&app_state.test_geometry);

                let test_render = GeometryRenderData {
                    model: Matrix4::identity(),
                    geometry: geo,
                };
                let mut geometries = Vec::new();
                geometries.push(test_render);
                let mut render_packet = RendererPacket {
                    delta_time: delta,
                    geometries: geometries,
                };
                Renderer::draw_frame(&mut render_packet)?;
                let elapsed_since_last_frame = last_frame_time.elapsed();
                if elapsed_since_last_frame < frame_duration {
                    thread::sleep(frame_duration - elapsed_since_last_frame);
                }

                last_frame_time = Instant::now();
            } else {
                break;
            }
        }
        Ok(())
    }

    pub fn shutdown() -> Result<()> {
        unsafe {
            if let Some(ref mut _state) = APP_STATE {
                APP_STATE = None;
                return Ok(());
            } else {
                return Err(AppError::AlreadyShutdown);
            }
        }
    }
}

impl<'a> Drop for ApplicationState<'a> {
    fn drop(&mut self) {
        unsafe {
            if let Some(ref mut _state) = APP_STATE {
                // probably log to console
                let _ = MaterialSystem::shutdown();
                let _ = TextureSystem::shutdown();
                let _ = Renderer::shutdown();
                let _ = EventState::shutdown();
                let _ = InputState::shutdown();
            }
        }
    }
}

fn application_on_event(
    code: usize,
    _sender: *const c_void,
    _listener_inst: *const c_void,
    _context: &EventCtx,
) -> bool {
    match EventCodes::from(code) {
        EventCodes::ApplicationQuit => unsafe {
            if let Some(ref mut state) = APP_STATE {
                println!("in application on event");
                state.is_running = false;
                return true;
            } else {
                return false;
            }
        },
        _ => return false,
    }
}

fn application_on_resize(
    code: usize,
    _sender: *const c_void,
    _listener_inst: *const c_void,
    context: &EventCtx,
) -> bool {
    match EventCodes::from(code) {
        EventCodes::WindowResized => {
            let arr = match context {
                EventCtx::I32(arr) => arr,
                _ => return false,
            };
            println!(
                "in application on resize: width {} height {} x {} y {}",
                arr[0], arr[1], arr[2], arr[3]
            );

            match Renderer::on_resize(arr[0], arr[1]) {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        _ => return false,
    }
}

pub fn on_event_debug(
    _code: usize,
    _sender: *const std::ffi::c_void,
    _listener: *const std::ffi::c_void,
    _ctx: &EventCtx,
) -> bool {
    let state = unsafe {
        if let Some(ref mut state) = APP_STATE {
            state
        } else {
            return false;
        }
    };
    let names = ["brick-wall", "door", "stone-wall", "tile"];
    static mut CHOICE: usize = 3;
    let old_name = unsafe { names[CHOICE] };
    unsafe {
        CHOICE += 1;
        CHOICE %= 4;
    }

    state
        .test_geometry
        .borrow_mut()
        .as_mut()
        .unwrap()
        .material
        .as_mut()
        .unwrap()
        .diffuse_map
        .texture = match TextureSystem::acquire(unsafe { names[CHOICE].to_string() }, true) {
        Ok(t) => Some(t),
        Err(_) => return false,
    };

    match TextureSystem::release(old_name) {
        Ok(_) => true,
        Err(_) => false,
    }
}
