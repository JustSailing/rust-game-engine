#[path = "./basic/mod.rs"]
pub mod basic;
#[path = "./renderer/mod.rs"]
pub mod renderer;

#[path = "./resources/mod.rs"]
pub mod resources;

#[path = "./systems/mod.rs"]
pub mod systems;

use crate::Game;
use crate::application::basic::math::matrix4::Matrix4;
use crate::application::renderer::renderer_types::GeometryRenderData;
use crate::application::resources::resource_types::Geometry;
use crate::application::systems::geometry_system::{GeometrySysConfig, GeometrySystem};
use crate::application::systems::material_system::{MaterialSysConfig, MaterialSystem};
use crate::application::systems::texture_system::{TextureSysConfig, TextureSystem};

use basic::event::{EventCodes, EventCtx, EventState};
use basic::input::InputState;
use basic::window::Window;
use renderer::renderer_types::{Renderer, RendererPacket};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};
use std::{ffi::c_void, fmt, ptr};

#[derive(Clone, Copy)]
pub struct AppConfig {
    pub start_pos_x: i32,
    pub start_pos_y: i32,
    pub start_width: i32,
    pub start_height: i32,
    pub name: &'static str,
}

#[derive(Debug)]
pub enum Error {
    CouldNotCreateWindow,
    NotInitialized,
    AlreadyInitialized,
    AlreadyShutdown,
    CouldNotInitializeGame,
    CouldNotUpdateGame,
    CouldNotRenderGame,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyInitialized => {
                write!(
                    f,
                    "Application State Already Initialized {}  {}",
                    file!(),
                    line!()
                )
            }
            Error::NotInitialized => {
                write!(
                    f,
                    "Application State Not Initialized {}  {}",
                    file!(),
                    line!()
                )
            }
            Error::AlreadyShutdown => {
                write!(
                    f,
                    "Application State Already Shutdown {}  {}",
                    file!(),
                    line!()
                )
            }
            Error::CouldNotCreateWindow => {
                write!(
                    f,
                    "Application state could not create window {}  {}",
                    file!(),
                    line!()
                )
            }
            Error::CouldNotInitializeGame => {
                write!(
                    f,
                    "Application state could not intialize game {}  {}",
                    file!(),
                    line!()
                )
            }
            Error::CouldNotUpdateGame => {
                write!(
                    f,
                    "Application state could not update game {}  {}",
                    file!(),
                    line!()
                )
            }
            Error::CouldNotRenderGame => {
                write!(
                    f,
                    "Application state could not render game {}  {}",
                    file!(),
                    line!()
                )
            }
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

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
                return Err(Error::AlreadyInitialized.into());
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

        EventState::register_event(
            EventCodes::KeyPressed as usize,
            ptr::null(),
            application_on_event,
        )?;

        EventState::register_event(
            EventCodes::KeyReleased as usize,
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
            return Err(Error::CouldNotInitializeGame.into());
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
                return Err(Error::NotInitialized.into());
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
                let delta = current_time.duration_since(last_frame_time);
                InputState::update(delta.as_secs_f32())?;
                if !(app_state.game.update)(&mut app_state.game, delta.as_secs_f32()) {
                    return Err(Error::CouldNotUpdateGame.into());
                }

                if !(app_state.game.render)(&mut app_state.game, delta.as_secs_f32()) {
                    return Err(Error::CouldNotRenderGame.into());
                }
                let geo = Rc::clone(&app_state.test_geometry);

                let test_render = GeometryRenderData {
                    model: Matrix4::identity(),
                    geometry: geo,
                };
                let mut geometries = Vec::new();
                geometries.push(test_render);
                let mut render_packet = RendererPacket {
                    delta_time: delta.as_secs_f32(),
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
                return Err(Error::AlreadyShutdown.into());
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
