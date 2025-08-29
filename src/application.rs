#[path = "./basic/mod.rs"]
mod basic;

use basic::window::{Event, Window};

pub struct AppConfig {
    pub start_pos_x: u32,
    pub start_pos_y: u32,
    pub start_width: u32,
    pub start_height: u32,
    pub name: &'static str,
}

static mut APP_STATE: Option<ApplicationState> = None;

pub struct ApplicationState {
    is_running: bool,
    is_suspended: bool,
    window: Window,
    pos_x: u32,
    pos_y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug)]
pub enum AppError {
    CouldNotGetDisplay,
    CouldNotCreateWindow,
    NotInitialized,
    AlreadyInitialized,
    StateError,
}

impl ApplicationState {
    pub fn create(config: &AppConfig) -> Result<(), AppError> {
        unsafe {
            if let Some(ref _a) = APP_STATE {
                return Err(AppError::AlreadyInitialized);
            }
        }

        let window = match Window::create(config.start_width, config.start_height) {
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

        Ok(())
    }

    pub fn run() -> Result<(), AppError> {
        let app_state = unsafe {
            if let Some(ref app) = APP_STATE {
                app
            } else {
                return Err(AppError::StateError);
            }
        };

        loop {
            if let Some(event) = app_state.window.get_event() {
                match event {
                    Event::Key { key } => continue,
                    Event::Button { button } => continue,
                    Event::MousePos { x, y } => continue,
                    Event::ConfigureNotify {
                        x,
                        y,
                        width,
                        height,
                    } => continue,
                    Event::CloseWindow => {
                        return Ok(());
                    }
                }
            }
        }
    }
}
