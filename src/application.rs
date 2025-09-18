#[path = "./basic/mod.rs"]
mod basic;
#[path = "./renderer/mod.rs"]
mod renderer;

use basic::event::{EventError, EventState};
use basic::input::{InputError, InputState};
use basic::window::{Event, Window};
use renderer::renderer_types::{FrontendRenderer, FrontendRendererError};


use crate::application::renderer::renderer_types::RendererPacket;

pub struct AppConfig {
    pub start_pos_x: i32,
    pub start_pos_y: i32,
    pub start_width: i32,
    pub start_height: i32,
    pub name: &'static str,
}

#[derive(Debug)]
pub enum AppError {
    CouldNotCreateWindow,
    NotInitialized,
    AlreadyInitialized,
    FrontendRendererAlreadyInitialized,
    FrontendRendererAlreadyShutdown,
    FrontendRendererNotInitialized,
    EventAlreadyInitialized,
    EventNotInitialized,
    EventAlreadyShutdown,
    InputAlreadyInitialized,
    InputNotInitialized,
    InputAlreadyShutdown,
    AlreadyShutdown,
    OperationFailed(&'static str),
}

impl From<FrontendRendererError> for AppError {
    fn from(value: FrontendRendererError) -> Self {
        match value {
            FrontendRendererError::AlreadyInitialized => {
                AppError::FrontendRendererAlreadyInitialized
            }
            FrontendRendererError::AlreadyShutdown => AppError::FrontendRendererAlreadyShutdown,
            FrontendRendererError::NotInitialized => AppError::FrontendRendererNotInitialized,
            FrontendRendererError::OperationFailed(v) => AppError::OperationFailed(v),
        }
    }
}

impl From<EventError> for AppError {
    fn from(value: EventError) -> Self {
        match value {
            EventError::AlreadyInitialized => AppError::EventAlreadyInitialized,
            EventError::NotInitialized => AppError::EventNotInitialized,
            EventError::AlreadyShutdown => AppError::EventAlreadyShutdown,
        }
    }
}

impl From<InputError> for AppError {
    fn from(value: InputError) -> Self {
        match value {
            InputError::AlreadyInitialized => AppError::InputAlreadyInitialized,
            InputError::AlreadyShutdown => AppError::InputAlreadyShutdown,
            InputError::NotInitialized => AppError::InputNotInitialized,
        }
    }
}

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
    pub fn create(config: &AppConfig) -> Result<(), AppError> {
        unsafe {
            if let Some(ref _a) = APP_STATE {
                return Err(AppError::AlreadyInitialized);
            }
        }

        let window = match Window::create(
            config.start_pos_x,
            config.start_pos_y,
            config.start_width,
            config.start_height,
        ) {
            Ok(w) => w,
            Err(_) => return Err(AppError::CouldNotCreateWindow),
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
                return Err(AppError::OperationFailed("Could not get Application State"));
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
                return Err(AppError::from(e));
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
                return Err(AppError::from(e));
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
                return Err(AppError::from(e));
            }
        }

        Ok(())
    }

    pub fn run() -> Result<(), AppError> {
        let app_state = unsafe {
            if let Some(ref mut app) = APP_STATE {
                app
            } else {
                return Err(AppError::NotInitialized);
            }
        };

        app_state.is_running = true;
        app_state.is_suspended = false;

        loop {
            if let Some(event) = app_state.window.get_event() {
                match event {
                    Event::Key { key: _ } => continue,
                    Event::Button { button: _ } => continue,
                    Event::MousePos { x: _, y: _ } => continue,
                    Event::ConfigureNotify {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        app_state.height = height;
                        app_state.width = width;
                        app_state.pos_x = x;
                        app_state.pos_y = y;
                        match FrontendRenderer::on_resize(width, height) {
                            Ok(_) => {},
                            Err(e) => return  Err(AppError::from(e)),
                        }
                    }
                    Event::CloseWindow => {
                        app_state.is_running = false;
                        app_state.is_suspended = false;
                        return Ok(());
                    }
                }
            }
            let render_packet = RendererPacket { delta_time: 1.0 };
            match FrontendRenderer::draw_frame(&render_packet) {
                Ok(_) => (),
                Err(e) => return Err(AppError::from(e)),
            }
        }
    }

    pub fn shutdown() -> Result<(), AppError> {
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
