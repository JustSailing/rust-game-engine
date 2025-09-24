#[path = "./basic/mod.rs"]
mod basic;
#[path = "./renderer/mod.rs"]
mod renderer;

use basic::event::EventState;
use basic::input::InputState;
use basic::math::{matrix4::Matrix4, vec3::Vec3};
use basic::window::Window;
use renderer::renderer_types::{FrontendRenderer, RendererPacket};

use crate::application::basic::math::consts;
use crate::application::basic::window::Key;

use std::fmt;

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
    OperationFailed(String),
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
            Error::OperationFailed(e) => {
                write!(f, "{e} {}  {}", file!(), line!())
            }
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

static mut APP_STATE: Option<ApplicationState> = None;

pub struct ApplicationState {
    is_running: bool,
    is_suspended: bool,
    window: Window,
    pos_x: i32,
    pos_y: i32,
    width: i32,
    height: i32,
}

impl ApplicationState {
    pub fn create(config: &AppConfig) -> Result<()> {
        unsafe {
            if let Some(ref _a) = APP_STATE {
                return Err(Error::AlreadyInitialized.into());
            }
        }

        let window = match Window::create(
            config.start_pos_x,
            config.start_pos_y,
            config.start_width,
            config.start_height,
        ) {
            Ok(w) => w,
            Err(_) => return Err(Error::CouldNotCreateWindow.into()),
        };
        window.set_title(config.name);
        window.show();

        unsafe {
            APP_STATE = Some(ApplicationState {
                is_running: false,
                is_suspended: false,
                window: window,
                pos_x: config.start_pos_x,
                pos_y: config.start_pos_y,
                width: config.start_width,
                height: config.start_height,
            });
        }

        let state = unsafe {
            if let Some(ref st) = APP_STATE {
                st
            } else {
                return Err(
                    Error::OperationFailed("Could not get Application State".into()).into(),
                );
            }
        };
        // for the error parts if initialization returns error not sure if I need if let part
        // may just APP_STATE= None would suffice
        match InputState::initialize() {
            Ok(_) => (),
            Err(e) => {
                unsafe {
                    if let Some(ref mut _state) = APP_STATE {
                        APP_STATE = None;
                    }
                }
                return Err(e);
            }
        }

        match EventState::initialize() {
            Ok(_) => (),
            Err(e) => {
                unsafe {
                    if let Some(ref mut _state) = APP_STATE {
                        APP_STATE = None;
                    }
                }
                return Err(e);
            }
        };

        match FrontendRenderer::initialize(config.name, &state.window) {
            Ok(_) => (),
            Err(e) => {
                unsafe {
                    if let Some(ref mut _state) = APP_STATE {
                        APP_STATE = None;
                    }
                }
                return Err(e);
            }
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

        loop {
            if app_state.window.get_event()? {
                let render_packet = RendererPacket { delta_time: 1.0 };
                FrontendRenderer::draw_frame(&render_packet)?;
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

impl Drop for ApplicationState {
    fn drop(&mut self) {
        unsafe {
            if let Some(ref mut _state) = APP_STATE {
                // probably log to console
                let _ = FrontendRenderer::shutdown();
                let _ = EventState::shutdown();
                let _ = InputState::shutdown();
            }
        }
    }
}

pub struct GameState {
    delta_time: f32,
    view: Matrix4,
    camera_position: Vec3,
    camera_euler: Vec3,
    view_dirty: bool,
}

pub struct Game {
    config: AppConfig,
    state: GameState,
    initialize: fn(game: &Game) -> bool,
    update: fn(game: &Game, delta: f32) -> bool,
    render: fn(game: &Game, delta: f32) -> bool,
    on_resize: fn(game: &Game, width: u32, height: u32) -> bool,
}

pub fn game_initialize(game: &mut Game) -> bool {
    game.state.camera_position = Vec3::new(0.0, 0.0, -30.0);
    game.state.view = Matrix4::translation(&game.state.camera_position);
    return true;
}

pub fn game_update(game: &mut Game, delta: f32) -> bool {
    if InputState::is_key_down(Key::A).unwrap() {
        camera_yaw(&mut game.state, 1.0 * delta);
    }

    if InputState::is_key_down(Key::D).unwrap() {
        camera_yaw(&mut game.state, -1.0 * delta);
    }

    recalculate_view(&mut game.state);
    return true;
}

pub fn game_render(game: &Game, delta: f32) -> bool {
    return true;
}

pub fn game_on_resize(game: &Game, width: u32, height: u32) {}

fn recalculate_view(state: &mut GameState) {
    if state.view_dirty {
        let rotation = Matrix4::euler_xyz(
            state.camera_euler.data[0],
            state.camera_euler.data[1],
            state.camera_euler.data[2],
        );

        let translation = Matrix4::translation(&state.camera_position);
        state.view = rotation * translation;
        state.view_dirty = false;
    }
}

fn camera_yaw(state: &mut GameState, amount: f32) {
    state.camera_euler.data[1] += amount; // y axis
    state.view_dirty = true;
}

fn camera_pitch(state: &mut GameState, amount: f32) {
    state.camera_euler.data[0] += amount;
    let limit = consts::deg_to_rad(89.0);
    state.camera_euler.data[0] = state.camera_euler.data[0].min(limit).max(-limit);
    state.view_dirty = true;
}
