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
use crate::application::basic::math::vec2::Vec2;
use crate::application::basic::math::vec3::{Vec3, Vector2D};
use crate::application::systems::geometry_system::GeometryConfig;
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
use systems::resource_system::{ResourceSysConfig, ResourceSysError, ResourceSystem};
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
    #[error("app error: could not create window {file} {line}")]
    CouldNotCreateWindow { file: &'static str, line: u32 },
    #[error("app error: already initialized {file} {line}")]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("app error: already shutdown {file} {line}")]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("app error: not initialized {file} {line}")]
    NotInitialized { file: &'static str, line: u32 },
    #[error("app error: could not initialize game {file} {line}")]
    CouldNotInitializeGame { file: &'static str, line: u32 },
    #[error("app error:  could not update game {file} {line}")]
    CouldNotUpdateGame { file: &'static str, line: u32 },
    #[error("app error:  could not render game {file} {line}")]
    CouldNotRenderGame { file: &'static str, line: u32 },
    #[error("{source}\napp error:  error from event system ")]
    EventSysError {
        source: EventSysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from input system {file} {line}")]
    InputSysError {
        source: InputSysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from window {file} {line}")]
    WindowError {
        source: WindowError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from renderer frontend {file} {line}")]
    FrontendRendererError {
        source: FrontendRendererError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from material system {file} {line}")]
    MaterialSysError {
        source: MaterialSysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from texture system {file} {line}")]
    TextureSysError {
        source: TextureSysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from geometry system {file} {line}")]
    GeometrySysError {
        source: GeometrySysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\napp error:  error from resource system {file} {line}")]
    ResourceSysError {
        source: ResourceSysError,
        file: &'static str,
        line: u32,
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
    test_geometry: Rc<RefCell<&'a mut Geometry<'a>>>,
    test_ui_geometry: Rc<RefCell<&'a mut Geometry<'a>>>,
}

static mut APP_STATE: Option<ApplicationState> = None;

impl<'a: 'static> ApplicationState<'a> {
    pub fn create(game: &mut Game) -> Result<()> {
        unsafe {
            if let Some(ref _a) = APP_STATE {
                return Err(AppError::AlreadyInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
        let app_config = game.config;
        let window = Window::create(
            app_config.start_pos_x,
            app_config.start_pos_y,
            app_config.start_width,
            app_config.start_height,
        )
        .map_err(|e| AppError::WindowError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        window.set_title(app_config.name);
        window.show();

        let resource_sys_config = ResourceSysConfig {
            max_loader_count: 32,
            asset_base_path: "assets".to_string(),
        };
        ResourceSystem::initialize(resource_sys_config).map_err(|e| {
            AppError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        InputState::initialize().map_err(|e| AppError::InputSysError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        EventState::initialize().map_err(|e| AppError::EventSysError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        EventState::register_event(
            EventCodes::ApplicationQuit as usize,
            ptr::null(),
            application_on_event,
        )
        .map_err(|e| AppError::EventSysError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        EventState::register_event(
            EventCodes::WindowResized as usize,
            ptr::null(),
            application_on_resize,
        )
        .map_err(|e| AppError::EventSysError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        EventState::register_event(EventCodes::Debug0 as usize, ptr::null(), on_event_debug)
            .map_err(|e| AppError::EventSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Renderer::initialize(app_config.name, &window).map_err(|e| {
            AppError::FrontendRendererError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        let texture_sys_config: TextureSysConfig = TextureSysConfig { max_count: 100 };
        TextureSystem::initialize(texture_sys_config).map_err(|e| AppError::TextureSysError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        let material_sys_config: MaterialSysConfig = MaterialSysConfig { max_count: 100 };
        MaterialSystem::initialize(material_sys_config).map_err(|e| {
            AppError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        let geometry_sys_config: GeometrySysConfig = GeometrySysConfig { max_count: 100 };
        GeometrySystem::initialize(geometry_sys_config).map_err(|e| {
            AppError::GeometrySysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        if !(game.initialize)(game) {
            return Err(AppError::CouldNotInitializeGame {
                file: file!(),
                line: line!(),
            });
        }

        let f = 512.0;

        let verts_2d: [Vector2D; 4] = [
            Vector2D {
                position: Vec2::new_zeroes(),
                texcoord: Vec2::new_zeroes(),
            },
            Vector2D {
                position: Vec2::new(f, f),
                texcoord: Vec2::new_ones(),
            },
            Vector2D {
                position: Vec2::new(0.0, f),
                texcoord: Vec2::new(0.0, 1.0),
            },
            Vector2D {
                position: Vec2::new(f, 0.0),
                texcoord: Vec2::new(1.0, 0.0),
            },
        ];
        let indices_2d: [u32; 6] = [2, 1, 0, 3, 0, 1];
        let ui_config = GeometryConfig::<Vector2D, u32> {
            vertices: Vec::from(verts_2d),
            indices: Vec::from(indices_2d),
            name: String::from("test_ui_geometry"),
            material_name: String::from("test_ui"),
        };

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
                test_geometry: Rc::new(RefCell::new(
                    GeometrySystem::get_default_geometry().map_err(|e| {
                        AppError::GeometrySysError {
                            source: e,
                            file: file!(),
                            line: line!(),
                        }
                    })?,
                )),
                test_ui_geometry: Rc::new(RefCell::new(
                    GeometrySystem::acquire_from_config(ui_config, true).map_err(|e| {
                        AppError::GeometrySysError {
                            source: e,
                            file: file!(),
                            line: line!(),
                        }
                    })?,
                )),
            });
        }

        Ok(())
    }

    pub fn run() -> Result<()> {
        let app_state = unsafe {
            if let Some(ref mut app) = APP_STATE {
                app
            } else {
                return Err(AppError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        app_state.is_running = true;
        app_state.is_suspended = false;
        const FPS: f32 = 60.0;
        let frame_duration: Duration = Duration::from_secs_f32(1.0 / FPS);
        let mut last_frame_time = Instant::now();
        loop {
            if app_state
                .window
                .get_event()
                .map_err(|e| AppError::WindowError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?
            {
                let current_time = Instant::now();
                let delta = current_time.duration_since(last_frame_time).as_secs_f32() / 60.0;
                InputState::update(delta).map_err(|e| AppError::InputSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
                if !(app_state.game.update)(&mut app_state.game, delta) {
                    return Err(AppError::CouldNotUpdateGame {
                        file: file!(),
                        line: line!(),
                    });
                }

                if !(app_state.game.render)(&mut app_state.game, delta) {
                    return Err(AppError::CouldNotRenderGame {
                        file: file!(),
                        line: line!(),
                    });
                }
                let geo = Rc::clone(&app_state.test_geometry);

                let test_render = GeometryRenderData {
                    model: Matrix4::identity(),
                    geometry: geo,
                };
                let mut geometries = Vec::new();
                geometries.push(test_render);

                let test_ui_render = GeometryRenderData {
                    model: Matrix4::translation(&Vec3::new(0.0, 0.0, 0.0)),
                    geometry: Rc::clone(&app_state.test_ui_geometry),
                };
                let mut ui_geometries = Vec::new();
                ui_geometries.push(test_ui_render);

                let mut render_packet = RendererPacket {
                    delta_time: delta,
                    geometries: geometries,
                    ui_geometries: ui_geometries,
                };
                Renderer::draw_frame(&mut render_packet).map_err(|e| {
                    AppError::FrontendRendererError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })?;
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
                return Err(AppError::AlreadyShutdown {
                    file: file!(),
                    line: line!(),
                });
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
