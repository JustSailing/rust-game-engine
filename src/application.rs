#[path = "basic/mod.rs"]
mod basic;

use basic::event::{EventError, EventState};
use basic::input::{InputError, InputState};
use basic::window::{Event, Window};

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
    EventAlreadyInitialized,
    EventNotInitialized,
    EventAlreadyShutdown,
    InputAlreadyInitialized,
    InputNotInitialized,
    InputAlreadyShutdown,
    AlreadyShutdown,
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

        match InputState::initialize() {
            Ok(_) => (),
            Err(e) => return Err(AppError::from(e)),
        }

        match EventState::initialize() {
            Ok(_) => (),
            Err(e) => return Err(AppError::from(e)),
        };

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
                    }
                    Event::CloseWindow => {
                        app_state.is_running = false;
                        app_state.is_suspended = false;
                        match EventState::shutdown() {
                            Ok(_) => (),
                            Err(e) => return Err(AppError::from(e)),
                        }
                        match InputState::shutdown() {
                            Ok(_) => (),
                            Err(e) => return Err(AppError::from(e)),
                        }
                        return Ok(());
                    }
                }
            }
        }
    }
}
