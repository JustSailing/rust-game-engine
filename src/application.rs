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
use std::time::{Duration, Instant};
use std::{ptr, thread};
use thiserror::Error;

use crate::application::basic::math::vec2::Vec2;
use crate::application::basic::math::vec3::{Vec3, Vector2D, Vector3D};
use crate::application::renderer::vulkan::vulkan_backend::BuiltInRenderpass;
use crate::application::resources::resource_types::{GeometryConfig, ResourceData, ResourceType};

use crate::application::systems::material_system::{
    BUILTIN_SHADER_NAME_MATERIAL, BUILTIN_SHADER_NAME_UI,
};
use crate::application::systems::shader_system::{ShaderSysConfig, ShaderSysError, ShaderSystem};
use crate::{Game, GameState};
use basic::event::{EventCallback, EventCodes, EventSysError, EventSystem};
use basic::input::{InputState, InputSysError};
use basic::math::matrix4::Matrix4;
use basic::window::{Window, WindowError};
use renderer::renderer_types::{GeometryRenderData, Renderer, RendererError, RendererPacket};
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
    #[error("app error: operation failed: {issue} {file} {line}")]
    OperationFailed {
        issue: String,
        file: &'static str,
        line: u32,
    },
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
    RendererSysError {
        source: RendererError,
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
    #[error("{source}\napp error:  error from shader system {file} {line}")]
    ShaderSysError {
        source: ShaderSysError,
        file: &'static str,
        line: u32,
    },
}

type Result<T> = std::result::Result<T, AppError>;

pub struct ApplicationState<'a> {
    game: Rc<RefCell<Game>>,
    is_running: bool,
    is_suspended: bool,
    window: Window<'a>,
    pos_x: i32,
    pos_y: i32,
    width: i32,
    height: i32,
    test_geometry: Rc<RefCell<Geometry>>,
    test_ui_geometry: Rc<RefCell<Geometry>>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    renderer_system: Rc<RefCell<Renderer>>,
    texture_system: Rc<RefCell<TextureSystem>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    geometry_system: Rc<RefCell<GeometrySystem<'a>>>,
    input_system: Rc<RefCell<InputState<'a>>>,
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
    //temporary
    event_system: Rc<RefCell<EventSystem<'a>>>,
}

impl<'a> ApplicationState<'a> {
    pub fn create() -> Result<Self> {
        let app_config = AppConfig {
            start_pos_x: 0,
            start_pos_y: 0,
            start_width: 1280,
            start_height: 720,
            name: "Hello William",
        };

        let resource_sys_config = ResourceSysConfig {
            max_loader_count: 32,
            asset_base_path: "assets".to_string(),
        };
        let resource_system = Rc::new(RefCell::new(
            ResourceSystem::initialize(resource_sys_config).map_err(|e| {
                AppError::ResourceSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?,
        ));

        let event_system = Rc::new(RefCell::new(EventSystem::initialize().map_err(|e| {
            AppError::EventSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?));

        let input_system = Rc::new(RefCell::new(
            InputState::initialize(event_system.clone()).map_err(|e| AppError::InputSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?,
        ));

        let window = Window::create(
            app_config.start_pos_x,
            app_config.start_pos_y,
            app_config.start_width,
            app_config.start_height,
            input_system.clone(),
            event_system.clone(),
        )
        .map_err(|e| AppError::WindowError {
            source: e,
            file: file!(),
            line: line!(),
        })?;

        window.set_title(app_config.name);
        window.show();

        let renderer_system = Rc::new(RefCell::new(
            Renderer::initialize(app_config.name, &window, resource_system.clone()).map_err(
                |e| AppError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                },
            )?,
        ));

        let texture_sys_config: TextureSysConfig = TextureSysConfig { max_count: 100 };
        let texture_system = Rc::new(RefCell::new(
            TextureSystem::initialize(
                texture_sys_config,
                Rc::clone(&renderer_system),
                Rc::clone(&resource_system),
            )
            .map_err(|e| AppError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?,
        ));

        texture_system
            .borrow_mut()
            .create_default_texture()
            .map_err(|e| AppError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        renderer_system
            .borrow_mut()
            .set_default_texture(texture_system.borrow().get_default_texture().map_err(|e| {
                AppError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?)
            .map_err(|e| AppError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let shader_sys_config = ShaderSysConfig {
            max_shader_count: 1024,
            max_uniform_count: 128,
            max_global_textures: 31,
            max_instance_textures: 31,
        };
        let shader_system = Rc::new(RefCell::new(
            ShaderSystem::initialize(
                shader_sys_config,
                renderer_system.clone(),
                texture_system.clone(),
            )
            .map_err(|e| AppError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?,
        ));

        let material_shader = resource_system
            .borrow()
            .load(BUILTIN_SHADER_NAME_MATERIAL, ResourceType::Shader)
            .map_err(|e| AppError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        let material_shader_config = match material_shader.data {
            ResourceData::ShaderResourceData(ref shader_config) => shader_config,
            _ => {
                return Err(AppError::OperationFailed {
                    issue: "Wrong resource type. Expected: shader config".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };

        shader_system
            .borrow_mut()
            .create(material_shader_config)
            .map_err(|e| AppError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let ui_shader = resource_system
            .borrow()
            .load(BUILTIN_SHADER_NAME_UI, ResourceType::Shader)
            .map_err(|e| AppError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let ui_shader_config = match ui_shader.data {
            ResourceData::ShaderResourceData(ref shader_config) => shader_config,
            _ => {
                return Err(AppError::OperationFailed {
                    issue: "Wrong resource type. Expected: shader config".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };

        shader_system
            .borrow_mut()
            .create(ui_shader_config)
            .map_err(|e| AppError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let material_sys_config: MaterialSysConfig = MaterialSysConfig { max_count: 100 };
        let material_system = Rc::new(RefCell::new(
            MaterialSystem::initialize(
                material_sys_config,
                Rc::clone(&texture_system),
                Rc::clone(&renderer_system),
                Rc::clone(&resource_system),
                Rc::clone(&shader_system),
            )
            .map_err(|e| AppError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?,
        ));

        material_system
            .borrow_mut()
            .create_default_material()
            .map_err(|e| AppError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let geometry_sys_config: GeometrySysConfig = GeometrySysConfig { max_count: 100 };
        let geometry_system = Rc::new(RefCell::new(
            GeometrySystem::initialize(
                geometry_sys_config,
                Rc::clone(&renderer_system),
                Rc::clone(&material_system),
            )
            .map_err(|e| AppError::GeometrySysError {
                source: e,
                file: file!(),
                line: line!(),
            })?,
        ));

        geometry_system
            .borrow_mut()
            .create_default_geometries()
            .map_err(|e| AppError::GeometrySysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let game = Rc::new(RefCell::new(Game {
            config: app_config,
            state: GameState {
                delta_time: 0.0,
                view: Matrix4::new_zeros(),
                camera_position: Vec3::new_zeroes(),
                camera_euler: Vec3::new_zeroes(),
                view_dirty: false,
            },
            texture_system: texture_system.clone(),
            renderer_system: renderer_system.clone(),
            test_geometry: geometry_system
                .borrow()
                .get_default_geometry()
                .map_err(|e| AppError::GeometrySysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?,
        }));

        if !game.borrow_mut().initialize() {
            return Err(AppError::CouldNotInitializeGame {
                file: file!(),
                line: line!(),
            });
        }

        event_system
            .borrow_mut()
            .register_event(
                EventCodes::ApplicationQuit as usize,
                ptr::null(),
                Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
            )
            .map_err(|e| AppError::EventSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        event_system
            .borrow_mut()
            .register_event(
                EventCodes::WindowResized as usize,
                ptr::null(),
                Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
            )
            .map_err(|e| AppError::EventSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        event_system
            .borrow_mut()
            .register_event(
                EventCodes::Debug0 as usize,
                ptr::null(),
                Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
            )
            .map_err(|e| AppError::EventSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        event_system
            .borrow_mut()
            .register_event(
                1,
                ptr::null(),
                Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
            )
            .map_err(|e| AppError::EventSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

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

        let test_ui_geometry = geometry_system
            .borrow_mut()
            .acquire_from_config(ui_config, true)
            .map_err(|e| AppError::GeometrySysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        const FACTOR: f32 = 10.0;
        const VERT_COUNT: usize = 4;
        let verts: [Vector3D; VERT_COUNT] = [
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, -0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(0.0, 0.0),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, 0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(1.0, 1.0),
            },
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, 0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(0.0, 1.0),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, -0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(1.0, 0.0),
            },
        ];

        const INDEX_COUNT: usize = 6;
        let indices: [u32; INDEX_COUNT] = [0, 1, 2, 0, 3, 1];

        let material_config = GeometryConfig::<Vector3D, u32> {
            vertices: Vec::from(verts),
            indices: Vec::from(indices),
            name: String::from("test_geometry"),
            material_name: String::from("test_material"),
        };

        let test_geometry = geometry_system
            .borrow_mut()
            .acquire_from_config(material_config, true)
            .map_err(|e| AppError::GeometrySysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        let app_state = Self {
            game: game,
            is_running: false,
            is_suspended: false,
            window: window,
            pos_x: app_config.start_pos_x,
            pos_y: app_config.start_pos_y,
            width: app_config.start_width,
            height: app_config.start_height,
            test_geometry,
            test_ui_geometry,
            resource_system,
            renderer_system,
            texture_system,
            material_system,
            geometry_system,
            event_system,
            input_system,
            shader_system,
        };

        Ok(app_state)
    }

    pub fn run(&mut self) -> Result<()> {
        self.is_running = true;
        self.is_suspended = false;
        const FPS: f32 = 60.0;
        let frame_duration: Duration = Duration::from_secs_f32(1.0 / FPS);
        let mut last_frame_time = Instant::now();
        loop {
            if self.window.get_event().map_err(|e| AppError::WindowError {
                source: e,
                file: file!(),
                line: line!(),
            })? {
                let current_time = Instant::now();
                let delta = current_time.duration_since(last_frame_time).as_secs_f32() / 60.0;
                self.input_system.borrow_mut().update(delta).map_err(|e| {
                    AppError::InputSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })?;
                if !self.game.borrow_mut().update(
                    delta,
                    &self.input_system.borrow(),
                    &mut self.renderer_system.borrow_mut(),
                    &mut self.event_system.borrow_mut(),
                ) {
                    return Err(AppError::CouldNotUpdateGame {
                        file: file!(),
                        line: line!(),
                    });
                }

                if !self.game.borrow_mut().render(delta) {
                    return Err(AppError::CouldNotRenderGame {
                        file: file!(),
                        line: line!(),
                    });
                }

                let test_render = GeometryRenderData {
                    model: Matrix4::identity(),
                    geometry: Rc::clone(&self.test_geometry),
                };

                let test_ui_render = GeometryRenderData {
                    model: Matrix4::translation(&Vec3::new(0.0, 0.0, 0.0)),
                    geometry: Rc::clone(&self.test_ui_geometry),
                };

                let mut geometries = Vec::new();
                geometries.push(test_render.clone());

                let mut ui_geometries = Vec::new();
                ui_geometries.push(test_ui_render.clone());

                let mut render_packet = RendererPacket {
                    delta_time: delta,
                    geometries: geometries,
                    ui_geometries: ui_geometries,
                };

                match self
                    .renderer_system
                    .borrow_mut()
                    .begin_frame(&mut render_packet)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })? {
                    true => {}
                    false => continue,
                }

                self.renderer_system
                    .borrow_mut()
                    .begin_renderpass(BuiltInRenderpass::World)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.shader_system
                    .borrow_mut()
                    .use_by_id(test_render.geometry.borrow().material.borrow().shader_id)
                    .map_err(|e| AppError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow_mut()
                    .apply_global(
                        test_render.geometry.borrow().material.borrow().shader_id as u32,
                        &self.renderer_system.borrow().projection,
                        &self.renderer_system.borrow().view,
                    )
                    .map_err(|e| AppError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow()
                    .apply_instance(&test_render.geometry.borrow_mut().material.borrow())
                    .map_err(|e| AppError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow()
                    .apply_local(
                        &test_render.geometry.borrow_mut().material.borrow(),
                        &test_render.model,
                    )
                    .map_err(|e| AppError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.renderer_system
                    .borrow_mut()
                    .draw_geometry(&mut render_packet.geometries, render_packet.delta_time)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.renderer_system
                    .borrow_mut()
                    .end_renderpass(BuiltInRenderpass::World)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.renderer_system
                    .borrow_mut()
                    .begin_renderpass(BuiltInRenderpass::UI)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.shader_system
                    .borrow_mut()
                    .use_by_id(test_ui_render.geometry.borrow().material.borrow().shader_id)
                    .map_err(|e| AppError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow_mut()
                    .apply_global(
                        test_ui_render.geometry.borrow().material.borrow().shader_id as u32,
                        &self.renderer_system.borrow().ui_projection,
                        &self.renderer_system.borrow().ui_view,
                    )
                    .map_err(|e| AppError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow()
                    .apply_instance(&test_ui_render.geometry.borrow_mut().material.borrow())
                    .map_err(|e| AppError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow()
                    .apply_local(
                        &test_ui_render.geometry.borrow_mut().material.borrow(),
                        &test_ui_render.model,
                    )
                    .map_err(|e| AppError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.renderer_system
                    .borrow_mut()
                    .draw_geometry(&mut render_packet.ui_geometries, render_packet.delta_time)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.renderer_system
                    .borrow_mut()
                    .end_renderpass(BuiltInRenderpass::UI)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.renderer_system
                    .borrow_mut()
                    .end_frame(render_packet.delta_time)
                    .map_err(|e| AppError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
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
}
