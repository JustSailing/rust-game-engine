use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::basic::event::EventTuple;
use crate::basic::math::transform::Transform;
use crate::basic::math::vec2::Vec2;
use crate::basic::math::vec3::{Vec3, Vector2D};
use crate::basic::math::vec4::Quat;
use crate::resources::resource_types::{
    GeometryConfig, Mesh, ResourceData, ResourceFlags, ResourceType, Skybox, TextureFilter,
    TextureRepeat, TextureUse,
};

use crate::basic::event::{EventCodes, EventCtx, EventSysError, EventSystem};
use crate::basic::input::{InputState, InputSysError};
use crate::basic::window::{Window, WindowError};
use crate::renderer::frontend_renderer::{
    BUILTIN_SHADER_NAME_MATERIAL, BUILTIN_SHADER_NAME_SKYBOX, BUILTIN_SHADER_NAME_UI, Renderer,
    RendererError,
};
use crate::renderer::renderer_types::{
    MeshPacketData, PacketData, RenderViewConfig, RenderViewKnownType, RenderViewMatrixViewSource,
    RenderViewPassConfig, SkyboxPacketData,
};
use crate::systems::camera_system::{CameraSysConfig, CameraSysError, CameraSystem};
use crate::systems::geometry_system::{GeometrySysConfig, GeometrySysError, GeometrySystem};
use crate::systems::material_system::{MaterialSysConfig, MaterialSysError, MaterialSystem};
use crate::systems::render_view_system::{
    RenderViewSysConfig, RenderViewSysError, RenderViewSystem,
};
use crate::systems::resource_system::{ResourceSysConfig, ResourceSysError, ResourceSystem};
use crate::systems::shader_system::{ShaderSysConfig, ShaderSysError, ShaderSystem};
use crate::systems::texture_system::DEFAULT_TEXTURE_NAME;
use crate::systems::texture_system::{TextureSysConfig, TextureSysError, TextureSystem};
use crate::{Game, GameState};

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
    #[error(transparent)]
    RenderViewSysErr(#[from] RenderViewSysError),
}

type Result<T> = std::result::Result<T, AppError>;

pub struct ApplicationState<'a> {
    game: Rc<RefCell<Game<'a>>>,
    is_running: bool,
    is_suspended: bool,
    window: Window,
    pos_x: i32,
    pos_y: i32,
    width: i32,
    height: i32,
    world_view_handle: usize,
    ui_view_handle: usize,
    skybox_view_handle: usize,
    skybox: Skybox,
    ui_meshes: Vec<Mesh>,
    meshes: Vec<Mesh>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    render_view_system: Rc<RefCell<RenderViewSystem<'a>>>,
    renderer_system: Rc<RefCell<Renderer<'a>>>,
    texture_system: Rc<RefCell<TextureSystem<'a>>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    geometry_system: Rc<RefCell<GeometrySystem<'a>>>,
    camera_system: Rc<RefCell<CameraSystem>>,
    input_system: Rc<RefCell<InputState>>,
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
    event_receiver: Receiver<EventTuple>,
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

        let (tx, rx) = channel::<EventTuple>();

        let event_system = Rc::new(RefCell::new(EventSystem::initialize(tx)?));

        let input_system = Rc::new(RefCell::new(InputState::initialize()?));

        let window = Window::create(
            app_config.start_pos_x,
            app_config.start_pos_y,
            app_config.start_width,
            app_config.start_height,
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

        let material_shader = resource_system.borrow().load(
            BUILTIN_SHADER_NAME_MATERIAL,
            ResourceType::Shader,
            ResourceFlags::empty(),
        )?;
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

        let ui_shader = resource_system.borrow().load(
            BUILTIN_SHADER_NAME_UI,
            ResourceType::Shader,
            ResourceFlags::empty(),
        )?;

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

        let skybox_shader = resource_system.borrow().load(
            BUILTIN_SHADER_NAME_SKYBOX,
            ResourceType::Shader,
            ResourceFlags::empty(),
        )?;

        let skybox_shader_config = match skybox_shader.data {
            ResourceData::ShaderResourceData(ref shader_config) => shader_config,
            _ => {
                return Err(AppError::OperationFailed {
                    issue: "Wrong resource type. Expected: shader config".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let skybox_shader = shader_system.borrow_mut().create(skybox_shader_config)?;

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

        let render_view_sys_config = RenderViewSysConfig::default().max_count(251);
        let render_view_system = Rc::new(RefCell::new(RenderViewSystem::initialize(
            &render_view_sys_config,
            Rc::clone(&renderer_system),
            Rc::clone(&shader_system),
            Rc::clone(&camera_system),
            Rc::clone(&material_system),
            Rc::clone(&geometry_system),
            Rc::clone(&texture_system),
        )?));

        renderer_system
            .borrow_mut()
            .set_render_view_system(Rc::clone(&render_view_system));

        let world = RenderViewConfig::default()
            .known_type(RenderViewKnownType::World)
            .width(0)
            .height(0)
            .name("world")
            .passes(vec![
                RenderViewPassConfig::default().name("Renderpass.Builtin.World"),
            ])
            .view_matrix_source(RenderViewMatrixViewSource::SceneCamera);
        let world_view_handle = render_view_system.borrow_mut().create_view(&world)?;

        let skybox = RenderViewConfig::default()
            .known_type(RenderViewKnownType::Skybox)
            .width(0)
            .height(0)
            .name("skybox")
            .passes(vec![
                RenderViewPassConfig::default().name("Renderpass.Builtin.Skybox"),
            ])
            .view_matrix_source(RenderViewMatrixViewSource::SceneCamera);

        let skybox_view_handle = render_view_system.borrow_mut().create_view(&skybox)?;

        let ui = RenderViewConfig::default()
            .known_type(RenderViewKnownType::UI)
            .width(0)
            .height(0)
            .name("ui")
            .passes(vec![
                RenderViewPassConfig::default().name("Renderpass.Builtin.UI"),
            ])
            .view_matrix_source(RenderViewMatrixViewSource::SceneCamera);

        let ui_view_handle = render_view_system.borrow_mut().create_view(&ui)?;

        let game = Rc::new(RefCell::new(Game {
            config: app_config,
            state: GameState { delta_time: 0.0 },
            camera_system: Rc::clone(&camera_system),
            texture_system: Rc::clone(&texture_system),
            renderer_system: Rc::clone(&renderer_system),
            material_system: Rc::clone(&material_system),
            input_system: Rc::clone(&input_system),
            event_system: Rc::clone(&event_system),
        }));

        // NOTE: TEMPORARY CODE
        //
        let mut cube_map = Skybox::default();
        cube_map.cube_map.use_type = TextureUse::CubeMap;
        cube_map.cube_map.texture_name = "skybox".to_string();
        cube_map.cube_map.filter_minify = TextureFilter::Linear;
        cube_map.cube_map.filter_magnify = TextureFilter::Linear;
        cube_map.cube_map.repeat_u = TextureRepeat::ClampToEdge;
        cube_map.cube_map.repeat_v = TextureRepeat::ClampToEdge;
        cube_map.cube_map.repeat_w = TextureRepeat::ClampToEdge;

        renderer_system
            .borrow_mut()
            .acquire_texture_map_resources(&mut cube_map.cube_map)?;

        texture_system.borrow_mut().acquire_cube("skybox", true)?;

        let skybox_config = geometry_system.borrow_mut().generate_cube_config(
            10.0,
            10.0,
            10.0,
            1.0,
            1.0,
            "skybox_cube",
            "",
        )?;
        cube_map.geometry_handle = geometry_system
            .borrow_mut()
            .acquire_from_config(skybox_config, true)?;

        cube_map.instance_id = renderer_system
            .borrow_mut()
            .acquire_shader_instance_resources(
                shader_system
                    .borrow_mut()
                    .get_mut_shader_by_id(skybox_shader)?,
                &vec![&cube_map.cube_map],
            )? as usize;

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
            //shader_name: BUILTIN_SHADER_NAME_UI.to_string(),
        };

        let ui_meshes = vec![Mesh {
            geometries: vec![
                geometry_system
                    .borrow_mut()
                    .acquire_from_config(ui_config, true)?,
            ],
            transform: Rc::new(RefCell::new(Transform::create())),
        }];

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
            // BUILTIN_SHADER_NAME_MATERIAL,
        )?;

        cube_mesh.geometries.push(
            geometry_system
                .borrow_mut()
                .acquire_from_config(geo_config, true)?,
        );

        let mut cube_mesh2 = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos(Vec3::new(15.0, 0.0, 0.0)))),
        };

        Transform::set_parent(&cube_mesh2.transform, &cube_mesh.transform);

        let geo_config2 = geometry_system.borrow().generate_cube_config(
            5.0,
            5.0,
            5.0,
            1.0,
            1.0,
            "test_cube2",
            "test_material",
            // BUILTIN_SHADER_NAME_MATERIAL,
        )?;

        let middle_cube = geometry_system
            .borrow_mut()
            .acquire_from_config(geo_config2, true)?;

        cube_mesh2.geometries.push(middle_cube);

        let mut cube_mesh3 = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos(Vec3::new(7.0, 0.0, 0.0)))),
        };

        Transform::set_parent(&cube_mesh3.transform, &cube_mesh2.transform);

        let geo_config3 = geometry_system.borrow().generate_cube_config(
            2.0,
            2.0,
            2.0,
            1.0,
            1.0,
            "test_cube3",
            "test_material",
            // BUILTIN_SHADER_NAME_MATERIAL,
        )?;

        let little_cube = geometry_system
            .borrow_mut()
            .acquire_from_config(geo_config3, true)?;

        cube_mesh3.geometries.push(little_cube);

        let mut car_mesh = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos(Vec3::new(15.0, 0.0, 0.0)))),
        };

        let mut resource =
            resource_system
                .borrow()
                .load("falcon", ResourceType::Mesh, ResourceFlags::empty())?;

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
                Vec3::new(0.0, 0.0, 0.0),
                Quat::identity(),
                Vec3::new(5.0, 5.0, 5.0),
            ))),
        };

        let mut resource =
            resource_system
                .borrow()
                .load("scene", ResourceType::Mesh, ResourceFlags::empty())?;

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
        let mut st_george = Mesh {
            geometries: Vec::new(),
            transform: Rc::new(RefCell::new(Transform::from_pos_rot_scale(
                Vec3::new(80.0, 0.0, 0.0),
                Quat::identity(),
                Vec3::new(0.25, 0.25, 0.25),
            ))),
        };

        let mut resource =
            resource_system
                .borrow()
                .load("Georg_C", ResourceType::Mesh, ResourceFlags::empty())?;

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
            st_george.geometries.push(
                geometry_system
                    .borrow_mut()
                    .acquire_from_config(geo_config.clone(), true)?,
            );
        }
        resource_system.borrow_mut().unload(&mut resource)?;
        let meshes = vec![
            cube_mesh,
            cube_mesh2,
            cube_mesh3,
            car_mesh,
            sponza_mesh,
            st_george,
        ];

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
            world_view_handle,
            ui_view_handle,
            skybox_view_handle,
            skybox: cube_map,
            meshes: meshes,
            ui_meshes: ui_meshes,
            resource_system,
            render_view_system,
            renderer_system,
            texture_system,
            material_system,
            geometry_system,
            camera_system,
            event_receiver: rx,
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
        let mut skybox_packet_data = SkyboxPacketData {
            skybox: self.skybox.clone(),
        };
        let mut world_mesh_data = MeshPacketData {
            meshes: self.meshes.clone(),
        };
        let mut ui_mesh_data = MeshPacketData {
            meshes: self.ui_meshes.clone(),
        };
        'outer: loop {
            if self.window.get_event()? {
                let current_time = Instant::now();
                let delta = current_time.duration_since(last_frame_time).as_secs_f32();
                self.input_system.borrow_mut().update(delta)?;
                while let Ok((code, data)) = self.event_receiver.try_recv() {
                    if code == EventCodes::ApplicationQuit as usize {
                        self.is_running = false;
                        break 'outer;
                    }

                    if code == EventCodes::KeyReleased as usize {
                        let key = match data {
                            EventCtx::U16(d) => d[0],
                            _ => todo!(),
                        };
                        self.input_system.borrow_mut().process_key(key, false)?;
                    }

                    if code == EventCodes::KeyPressed as usize {
                        let key = match data {
                            EventCtx::U16(d) => d[0],
                            _ => todo!(),
                        };
                        self.input_system.borrow_mut().process_key(key, true)?;
                    }

                    if code == EventCodes::SetRenderMode as usize {
                        self.render_view_system
                            .borrow_mut()
                            .get_mut_render_view_by_id(self.world_view_handle)?
                            .handle_event(code, std::ptr::null(), std::ptr::null(), &data);
                    }

                    if code == EventCodes::WindowResized as usize {
                        match data {
                            EventCtx::I32(d) => {
                                self.render_view_system
                                    .borrow_mut()
                                    .on_window_resize(d[0] as u16, d[1] as u16)?;
                                self.renderer_system.borrow_mut().on_resize(d[0], d[1])?;
                            }
                            _ => todo!(),
                        }
                    }
                }
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

                self.camera_system
                    .borrow_mut()
                    .get_mut_default_camera()
                    .recalculate_view();

                match self.renderer_system.borrow_mut().begin_frame(delta)? {
                    true => {}
                    false => continue,
                }
                let frame_number = self.renderer_system.borrow().frame_number;

                let mut skybox_view_packet = self
                    .render_view_system
                    .borrow_mut()
                    .get_mut_render_view_by_id(self.skybox_view_handle)?
                    .build_packet(
                        &mut PacketData::Skybox(&mut skybox_packet_data),
                        &self.camera_system,
                        &self.geometry_system,
                        &self.material_system,
                        &self.texture_system,
                    )?;

                self.render_view_system.borrow().on_render(
                    self.skybox_view_handle,
                    frame_number,
                    &mut skybox_view_packet,
                )?;

                let mut world_view_packet = self
                    .render_view_system
                    .borrow_mut()
                    .get_mut_render_view_by_id(self.world_view_handle)?
                    .build_packet(
                        &mut PacketData::Mesh(&mut world_mesh_data),
                        &self.camera_system,
                        &self.geometry_system,
                        &self.material_system,
                        &self.texture_system,
                    )?;

                self.render_view_system.borrow().on_render(
                    self.world_view_handle,
                    frame_number,
                    &mut world_view_packet,
                )?;

                let mut ui_view_packet = self
                    .render_view_system
                    .borrow_mut()
                    .get_mut_render_view_by_id(self.ui_view_handle)?
                    .build_packet(
                        &mut PacketData::Mesh(&mut ui_mesh_data),
                        &self.camera_system,
                        &self.geometry_system,
                        &self.material_system,
                        &self.texture_system,
                    )?;

                self.render_view_system.borrow_mut().on_render(
                    self.ui_view_handle,
                    frame_number,
                    &mut ui_view_packet,
                )?;

                self.renderer_system.borrow_mut().end_frame(delta)?;

                let quat = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 100.0 * delta, false);

                self.meshes[0].transform.borrow_mut().rotate(quat);
                self.meshes[1].transform.borrow_mut().rotate(quat);
                // self.meshes[2].transform.borrow_mut().rotate(quat);

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
