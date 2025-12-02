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

use crate::application::basic::math::transform::Transform;
use crate::application::basic::math::vec2::Vec2;
use crate::application::basic::math::vec3::{Vec3, Vector2D};
use crate::application::basic::math::vec4::{Quat, Vec4};
use crate::application::resources::resource_types::{
    GeometryConfig, Mesh, ResourceData, ResourceType,
};

use crate::application::systems::camera_system::{CameraSysConfig, CameraSysError, CameraSystem};
use crate::application::systems::material_system::{
    BUILTIN_SHADER_NAME_MATERIAL, BUILTIN_SHADER_NAME_UI,
};
use crate::application::systems::shader_system::{ShaderSysConfig, ShaderSysError, ShaderSystem};
use crate::application::systems::texture_system::DEFAULT_TEXTURE_NAME;
use crate::{Game, GameState};
use basic::event::{EventCallback, EventCodes, EventSysError, EventSystem};
use basic::input::{InputState, InputSysError};
use basic::math::matrix4::Matrix4;
use basic::window::{Window, WindowError};
use renderer::frontend_renderer::{Renderer, RendererError};
use renderer::renderer_types::{GeometryRenderData, RendererPacket};
use resources::resource_types::GeometryHandle;
use systems::geometry_system::{GeometrySysConfig, GeometrySysError, GeometrySystem};
use systems::material_system::{MaterialSysConfig, MaterialSysError, MaterialSystem};
use systems::resource_system::{ResourceSysConfig, ResourceSysError, ResourceSystem};
use systems::texture_system::{TextureSysConfig, TextureSysError, TextureSystem};

#[derive(Debug, Default, Clone, Copy)]
pub struct AppConfig {
    pub start_pos_x: i32,
    pub start_pos_y: i32,
    pub start_width: i32,
    pub start_height: i32,
    pub name: &'static str,
}

impl AppConfig {
    pub fn start_pos_x(mut self, start_pos_x: i32) -> Self {
        self.start_pos_x = start_pos_x;
        self
    }

    pub fn start_pos_y(mut self, start_pos_y: i32) -> Self {
        self.start_pos_y = start_pos_y;
        self
    }

    pub fn start_width(mut self, start_width: i32) -> Self {
        self.start_width = start_width;
        self
    }

    pub fn start_height(mut self, start_height: i32) -> Self {
        self.start_height = start_height;
        self
    }

    pub fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }
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
    #[error(transparent)]
    EventSysErr(#[from] EventSysError),
    #[error(transparent)]
    InputSysErr(#[from] InputSysError),
    #[error(transparent)]
    WindowErr(#[from] WindowError),
    #[error(transparent)]
    TextureSysErr(#[from] TextureSysError),
    #[error(transparent)]
    MaterialSysErr(#[from] MaterialSysError),
    #[error(transparent)]
    RendererSysErr(#[from] RendererError),
    #[error(transparent)]
    ResourceSysErr(#[from] ResourceSysError),
    #[error(transparent)]
    ShaderSysErr(#[from] ShaderSysError),
    #[error(transparent)]
    GeometrySysErr(#[from] GeometrySysError),
    #[error(transparent)]
    CameraSysErr(#[from] CameraSysError),
}

type Result<T> = std::result::Result<T, AppError>;

pub struct ApplicationState<'a> {
    game: Rc<RefCell<Game<'a>>>,
    is_running: bool,
    is_suspended: bool,
    window: Window<'a>,
    pos_x: i32,
    pos_y: i32,
    width: i32,
    height: i32,
    test_ui_geometry: GeometryHandle,
    meshes: Vec<Mesh>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    renderer_system: Rc<RefCell<Renderer>>,
    texture_system: Rc<RefCell<TextureSystem>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    geometry_system: Rc<RefCell<GeometrySystem<'a>>>,
    camera_system: Rc<RefCell<CameraSystem>>,
    input_system: Rc<RefCell<InputState<'a>>>,
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
    event_system: Rc<RefCell<EventSystem<'a>>>,
}

impl<'a> ApplicationState<'a> {
    pub fn create() -> Result<Self> {
        let app_config = AppConfig::default()
            .start_pos_x(0)
            .start_pos_y(0)
            .start_width(1280)
            .start_height(720)
            .name("Hello William");

        let resource_sys_config = ResourceSysConfig::default()
            .max_loader_count(32)
            .asset_base_path("assets".to_string());

        let resource_system = Rc::new(RefCell::new(ResourceSystem::initialize(
            resource_sys_config,
        )?));

        let event_system = Rc::new(RefCell::new(EventSystem::initialize()?));

        let input_system = Rc::new(RefCell::new(InputState::initialize(event_system.clone())?));

        let window = Window::create(
            app_config.start_pos_x,
            app_config.start_pos_y,
            app_config.start_width,
            app_config.start_height,
            Rc::clone(&input_system),
            Rc::clone(&event_system),
        )?;

        window.set_title(app_config.name);
        window.show();

        let camera_sys_config = CameraSysConfig::default().max_count(61);
        let camera_system = Rc::new(RefCell::new(CameraSystem::initialize(&camera_sys_config)?));

        let texture_sys_config = TextureSysConfig::default().max_count(4096);
        let texture_system = Rc::new(RefCell::new(TextureSystem::initialize(
            texture_sys_config,
            Rc::clone(&resource_system),
        )?));

        let renderer_system = Rc::new(RefCell::new(Renderer::initialize(
            app_config.name,
            &window,
            Rc::clone(&resource_system),
            Rc::clone(&texture_system),
            Rc::clone(&camera_system),
        )?));

        // NOTE: This should be temporary when there's a better way for the swapchain to
        // have access to the texture system
        texture_system
            .borrow_mut()
            .set_renderer(Rc::clone(&renderer_system));

        texture_system.borrow_mut().create_default_textures()?;

        renderer_system.borrow_mut().set_default_texture(
            texture_system
                .borrow_mut()
                .acquire(DEFAULT_TEXTURE_NAME, true)?,
        )?;

        let shader_sys_config = ShaderSysConfig::default()
            .max_shader_count(1024)
            .max_uniform_count(128)
            .max_global_textures(31)
            .max_instance_textures(31);

        let shader_system = Rc::new(RefCell::new(ShaderSystem::initialize(
            shader_sys_config,
            Rc::clone(&renderer_system),
            Rc::clone(&texture_system),
        )?));

        let material_shader = resource_system
            .borrow()
            .load(BUILTIN_SHADER_NAME_MATERIAL, ResourceType::Shader)?;
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

        shader_system.borrow_mut().create(material_shader_config)?;

        let ui_shader = resource_system
            .borrow()
            .load(BUILTIN_SHADER_NAME_UI, ResourceType::Shader)?;

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

        shader_system.borrow_mut().create(ui_shader_config)?;

        let material_sys_config = MaterialSysConfig::default().max_count(4096);
        let material_system = Rc::new(RefCell::new(MaterialSystem::initialize(
            material_sys_config,
            Rc::clone(&texture_system),
            Rc::clone(&renderer_system),
            Rc::clone(&resource_system),
            Rc::clone(&shader_system),
        )?));

        material_system.borrow_mut().create_default_materials()?;

        let geometry_sys_config = GeometrySysConfig::default().max_count(4096);
        let geometry_system = Rc::new(RefCell::new(GeometrySystem::initialize(
            geometry_sys_config,
            Rc::clone(&renderer_system),
            Rc::clone(&material_system),
            Rc::clone(&shader_system),
        )?));

        geometry_system.borrow_mut().create_default_geometries()?;

        let game = Rc::new(RefCell::new(Game {
            config: app_config,
            state: GameState { delta_time: 0.0 },
            camera_system: Rc::clone(&camera_system),
            texture_system: Rc::clone(&texture_system),
            renderer_system: Rc::clone(&renderer_system),
            material_system: Rc::clone(&material_system),
            input_system: Rc::clone(&input_system),
        }));

        event_system.borrow_mut().register_event(
            EventCodes::ApplicationQuit as usize,
            ptr::null(),
            Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
        )?;

        event_system.borrow_mut().register_event(
            EventCodes::WindowResized as usize,
            ptr::null(),
            Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
        )?;

        event_system.borrow_mut().register_event(
            EventCodes::Debug0 as usize,
            ptr::null(),
            Box::new(Rc::clone(&game) as Rc<RefCell<dyn EventCallback>>),
        )?;

        let w = 256.0;
        let h = 128.0;
        let verts_2d: [Vector2D; 4] = [
            Vector2D {
                position: Vec2::new_zeroes(),
                coord: Vec2::new_zeroes(),
            },
            Vector2D {
                position: Vec2::new(w, h),
                coord: Vec2::new_ones(),
            },
            Vector2D {
                position: Vec2::new(0.0, h),
                coord: Vec2::new(0.0, 1.0),
            },
            Vector2D {
                position: Vec2::new(w, 0.0),
                coord: Vec2::new(1.0, 0.0),
            },
        ];
        let indices_2d: [u32; 6] = [2, 1, 0, 3, 0, 1];
        let ui_config = GeometryConfig::<Vector2D, u32> {
            vertices: Vec::from(verts_2d),
            indices: Vec::from(indices_2d),
            center: Default::default(),
            min_extents: Default::default(),
            max_extents: Default::default(),
            name: String::from("test_ui_geometry"),
            material_name: String::from("test_ui"),
        };

        let test_ui_geometry = geometry_system
            .borrow_mut()
            .acquire_from_config(ui_config, true)?;

        let mut cube_mesh = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::create())),
        };

        let geo_config = geometry_system.borrow().generate_cube_config(
            10.0,
            10.0,
            10.0,
            1.0,
            1.0,
            "test_cube",
            "test_material",
        )?;

        cube_mesh.geometries.push(
            geometry_system
                .borrow_mut()
                .acquire_from_config(geo_config, true)?,
        );

        let mut cube_mesh2 = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos(Vec3::new(15.0, 0.0, 1.0)))),
        };

        cube_mesh2
            .transform
            .borrow_mut()
            .set_parent(Rc::clone(&cube_mesh.transform));

        let geo_config2 = geometry_system.borrow().generate_cube_config(
            5.0,
            5.0,
            5.0,
            1.0,
            1.0,
            "test_cube2",
            "test_material",
        )?;

        let little_cube = geometry_system
            .borrow_mut()
            .acquire_from_config(geo_config2, true)?;

        cube_mesh2.geometries.push(little_cube);

        let mut cube_mesh3 = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos(Vec3::new(7.5, 0.0, 1.0)))),
        };

        cube_mesh3
            .transform
            .borrow_mut()
            .set_parent(Rc::clone(&cube_mesh2.transform));

        let geo_config3 = geometry_system.borrow().generate_cube_config(
            2.0,
            2.0,
            2.0,
            1.0,
            1.0,
            "test_cube3",
            "test_material",
        )?;

        let little_cube = geometry_system
            .borrow_mut()
            .acquire_from_config(geo_config3, true)?;

        cube_mesh3.geometries.push(little_cube);

        cube_mesh2
            .transform
            .borrow_mut()
            .set_parent(Rc::clone(&cube_mesh.transform));

        cube_mesh3
            .transform
            .borrow_mut()
            .set_parent(Rc::clone(&cube_mesh2.transform));

        let mut car_mesh = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos(Vec3::new(15.0, 0.0, 1.0)))),
        };

        let mut resource = resource_system
            .borrow()
            .load("falcon", ResourceType::Mesh)?;

        let geometry_configs = match resource.data {
            ResourceData::MeshResourceData(ref mut geometry_configs) => geometry_configs,
            _ => {
                return Err(AppError::OperationFailed {
                    issue: "wrong resource data for falcon".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };

        for geo_config in geometry_configs.iter_mut() {
            car_mesh.geometries.push(
                geometry_system
                    .borrow_mut()
                    .acquire_from_config(geo_config.clone(), true)?,
            );
        }
        resource_system.borrow_mut().unload(&mut resource)?;

        let mut sponza_mesh = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos_rot_scale(
                Vec3::new(15.0, 0.0, 1.0),
                Quat::identity(),
                Vec3::new(0.05, 0.05, 0.05),
            ))),
        };

        let mut resource = resource_system
            .borrow()
            .load("sponza", ResourceType::Mesh)?;

        let geometry_configs = match resource.data {
            ResourceData::MeshResourceData(ref mut geometry_configs) => geometry_configs,
            _ => {
                return Err(AppError::OperationFailed {
                    issue: "wrong resource data for falcon".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };

        for geo_config in geometry_configs.iter_mut() {
            sponza_mesh.geometries.push(
                geometry_system
                    .borrow_mut()
                    .acquire_from_config(geo_config.clone(), true)?,
            );
        }
        resource_system.borrow_mut().unload(&mut resource)?;
        let meshes = vec![cube_mesh, cube_mesh2, cube_mesh3, car_mesh, sponza_mesh];

        if !game.borrow_mut().initialize() {
            return Err(AppError::CouldNotInitializeGame {
                file: file!(),
                line: line!(),
            });
        }
        let app_state = Self {
            game,
            is_running: false,
            is_suspended: false,
            window,
            pos_x: app_config.start_pos_x,
            pos_y: app_config.start_pos_y,
            width: app_config.start_width,
            height: app_config.start_height,
            meshes,
            test_ui_geometry,
            resource_system,
            renderer_system,
            texture_system,
            material_system,
            geometry_system,
            camera_system,
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

        let world_renderpass_name = String::from("Renderpass.Builtin.World");
        let ui_renderpass_name = String::from("Renderpass.Builtin.UI");
        loop {
            if self.window.get_event()? {
                let current_time = Instant::now();
                let delta = current_time.duration_since(last_frame_time).as_secs_f32();
                self.input_system.borrow_mut().update(delta)?;
                if !self.game.borrow_mut().update(delta, &mut self.meshes) {
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

                let geo_sys = self.geometry_system.borrow_mut();

                let mut test_ui_render = GeometryRenderData {
                    model: Matrix4::translation(&Vec3::new(0.0, 0.0, 0.0)),
                    geometry: geo_sys.get_geometry(self.test_ui_geometry)?,
                };

                // shouldn't be creating a new vec each iteration
                let geometries = Vec::new();

                let mut ui_geometries = Vec::new();
                ui_geometries.push(test_ui_render.clone());

                let mut render_packet = RendererPacket {
                    delta_time: delta,
                    geometries,
                    ui_geometries,
                };

                match self
                    .renderer_system
                    .borrow_mut()
                    .begin_frame(&mut render_packet)?
                {
                    true => {}
                    false => continue,
                }

                self.renderer_system
                    .borrow_mut()
                    .begin_renderpass(&world_renderpass_name)?;

                let quat = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 100.0 * delta, false);

                self.meshes[0].transform.borrow_mut().rotate(quat);
                self.meshes[1].transform.borrow_mut().rotate(quat);
                self.meshes[2].transform.borrow_mut().rotate(quat);

                let mut global_updated = false;

                for mesh in self.meshes.iter() {
                    for g in mesh.geometries.iter() {
                        //let g_ref = g.borrow_mut();
                        let g_ref = geo_sys.get_geometry(*g)?;
                        {
                            let mut material_sys = self.material_system.borrow_mut();
                            let material_ref =
                                material_sys.get_mut_material(g_ref.material_handle)?;

                            self.shader_system
                                .borrow_mut()
                                .use_by_id(material_ref.shader_id)?;
                        }
                        if !global_updated {
                            let material_sys = self.material_system.borrow();
                            let material_ref = material_sys.get_material(g_ref.material_handle)?;
                            let mut camera_system = self.camera_system.borrow_mut();
                            let camera = camera_system.get_mut_default_camera();
                            camera.recalculate_view();
                            material_sys.apply_global(
                                material_ref.shader_id as u32,
                                &self.renderer_system.borrow().projection,
                                camera.get_view(),
                                camera.get_position(),
                                &self.renderer_system.borrow().ambient_colour,
                                self.renderer_system.borrow().render_mode as u32,
                            )?;
                            global_updated = true;
                        }

                        let mut material_render_frame_number_equal_render_system = false;
                        let w = mesh.transform.borrow_mut().get_world();
                        {
                            let mut material_sys = self.material_system.borrow_mut();
                            material_render_frame_number_equal_render_system = material_sys
                                .get_mut_material(g_ref.material_handle)?
                                .render_frame_number
                                != self.renderer_system.borrow().frame_number;
                        }
                        if material_render_frame_number_equal_render_system {
                            let material_sys = self.material_system.borrow();
                            let material_ref = material_sys.get_material(g_ref.material_handle)?;
                            material_sys.apply_instance(
                                material_ref,
                                g_ref.material_instance_id,
                                &w,
                            )?;
                        } else {
                            let mut material_sys = self.material_system.borrow_mut();
                            let material_ref =
                                material_sys.get_mut_material(g_ref.material_handle)?;
                            material_ref.render_frame_number =
                                self.renderer_system.borrow().frame_number;
                        }

                        let mut geo_render_data = GeometryRenderData {
                            model: w, //mesh.transform.borrow_mut().get_world(),
                            geometry: g_ref,
                        };

                        self.renderer_system
                            .borrow_mut()
                            .draw_geometry(&mut geo_render_data, render_packet.delta_time)?;
                        render_packet.geometries.push(geo_render_data);
                    }
                }

                self.renderer_system.borrow_mut().end_renderpass()?;

                self.renderer_system
                    .borrow_mut()
                    .begin_renderpass(&ui_renderpass_name)?;
                let material_sys = self.material_system.borrow();
                let material_ref =
                    material_sys.get_material(test_ui_render.geometry.material_handle)?;

                self.shader_system
                    .borrow_mut()
                    .use_by_id(material_ref.shader_id)?;
                material_sys.apply_global(
                    material_ref.shader_id as u32,
                    &self.renderer_system.borrow().ui_projection,
                    &self.renderer_system.borrow().ui_view,
                    &Vec3::new(1.0, 1.0, 1.0),
                    &Vec4::new(1.0, 1.0, 1.0, 1.0),
                    0,
                )?;
                {
                    material_sys.apply_instance(
                        material_ref,
                        test_ui_render.geometry.material_instance_id,
                        &test_ui_render.model,
                    )?;
                }
                self.renderer_system
                    .borrow_mut()
                    .draw_geometry(&mut test_ui_render, render_packet.delta_time)?;

                self.renderer_system.borrow_mut().end_renderpass()?;

                self.renderer_system
                    .borrow_mut()
                    .end_frame(render_packet.delta_time)?;

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
