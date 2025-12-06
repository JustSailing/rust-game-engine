mod application;
mod basic;
mod renderer;
mod resources;
mod systems;

use std::cell::RefCell;
use std::ptr;
use std::rc::Rc;

use application::{AppConfig, ApplicationState};
use basic::math::vec3::Vec3;

use crate::basic::event::{EventCodes, EventCtx, EventSystem};
use crate::basic::input::InputState;
use crate::basic::window::Key;
use crate::renderer::frontend_renderer::Renderer;
use crate::renderer::renderer_types::RendererDebugViewMode;
use crate::resources::resource_types::Mesh;
use crate::systems::camera_system::CameraSystem;
use crate::systems::material_system::MaterialSystem;
use crate::systems::texture_system::TextureSystem;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Hello, world!");

    let app = Rc::new(RefCell::new(ApplicationState::create()?));

    let _ = app.borrow_mut().run()?;
    Ok(())
}

#[derive(Clone, Copy)]
pub struct GameState {
    delta_time: f32,
    // using default_camera from teh camera_system
    // world_camera: CameraHandle,
}

#[derive(Clone)]
pub struct Game<'a> {
    config: AppConfig,
    state: GameState,
    texture_system: Rc<RefCell<TextureSystem<'a>>>,
    renderer_system: Rc<RefCell<Renderer<'a>>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    camera_system: Rc<RefCell<CameraSystem>>,
    input_system: Rc<RefCell<InputState>>,
    event_system: Rc<RefCell<EventSystem>>,
}

impl<'a> Game<'a> {
    // temporary
    pub fn initialize(&mut self) -> bool {
        self.camera_system
            .borrow_mut()
            .get_mut_default_camera()
            .set_position(&Vec3::new(10.5, 5.0, 5.5));
        true
    }

    pub fn update(&mut self, delta: f32, _meshes: &mut Vec<Mesh>) -> bool {
        let movement = 8000.0;

        let mut cam_sys = self.camera_system.borrow_mut();
        let camera = cam_sys.get_mut_default_camera();

        let input_state = self.input_system.borrow();

        if input_state.is_key_down(Key::A).unwrap() {
            camera.yaw(1.0 * delta * movement * 0.9);
        }

        if input_state.is_key_down(Key::D).unwrap() {
            camera.yaw(-1.0 * delta * movement * 0.9);
        }
        if input_state.is_key_down(Key::Up).unwrap() {
            camera.pitch(1.0 * delta * movement);
        }

        if input_state.is_key_down(Key::Down).unwrap() {
            camera.pitch(-1.0 * delta * movement);
        }

        if input_state.is_key_down(Key::W).unwrap() {
            camera.move_forward(movement * delta);
        }

        if input_state.is_key_down(Key::S).unwrap() {
            camera.move_backward(movement * delta);
        }

        if input_state.is_key_down(Key::Q).unwrap() {
            camera.move_left(movement * delta);
        }

        if input_state.is_key_down(Key::E).unwrap() {
            camera.move_right(movement * delta);
        }

        if input_state.is_key_down(Key::Z).unwrap() {
            camera.move_up(movement * delta);
        }

        if input_state.is_key_down(Key::C).unwrap() {
            camera.move_down(movement * delta);
        }

        if input_state.is_key_down(Key::T).unwrap() {
            // let names = ["brick_wall", "door", "stone_wall", "tile", "test_material"];
            // static mut CHOICE: usize = 4;
            // let old_name = unsafe { names[CHOICE] };
            // unsafe {
            //     CHOICE += 1;
            //     CHOICE %= 5;
            // }

            // let new_name = unsafe { names[CHOICE].to_string() };

            // let g = &meshes[0].geometries[0];
            // let mut material_config = MaterialConfig::default().name(&new_name).auto_release(true);
            // let mut instance_id = INVALID_ID;
            // (g.borrow_mut().material, instance_id) = match self
            //     .material_system
            //     .borrow_mut()
            //     .acquire(&mut material_config)
            // {
            //     Ok((material, instance_id)) => (material, instance_id),
            //     Err(_) => return false,
            // };

            // g.borrow_mut().material_instance_id = instance_id;

            // let _ = self.material_system.borrow_mut().release(old_name);

            // let g2 = &meshes[1].geometries[0];
            // material_config = MaterialConfig::default().name(&new_name);
            // instance_id = INVALID_ID;
            // (g2.borrow_mut().material, instance_id) = match self
            //     .material_system
            //     .borrow_mut()
            //     .acquire(&mut material_config)
            // {
            //     Ok((material, instance_id)) => (material, instance_id),
            //     Err(_) => return false,
            // };

            // g.borrow_mut().material_instance_id = instance_id;

            // let _ = self.material_system.borrow_mut().release(old_name);
        }

        if input_state.is_key_down(Key::_1).unwrap() {
            let _ = self.event_system.borrow_mut().fire_event(
                EventCodes::SetRenderMode as usize,
                ptr::null(),
                &EventCtx::U32([RendererDebugViewMode::Lighting as u32, 0, 0, 0]),
            );
        }

        if input_state.is_key_down(Key::_2).unwrap() {
            let _ = self.event_system.borrow_mut().fire_event(
                EventCodes::SetRenderMode as usize,
                ptr::null(),
                &EventCtx::U32([RendererDebugViewMode::Normals as u32, 0, 0, 0]),
            );
        }

        if input_state.is_key_down(Key::_0).unwrap() {
            let _ = self.event_system.borrow_mut().fire_event(
                EventCodes::SetRenderMode as usize,
                ptr::null(),
                &EventCtx::U32([RendererDebugViewMode::Default as u32, 0, 0, 0]),
            );
        }

        camera.recalculate_view();
        true
    }

    pub fn render(&self, _delta: f32) -> bool {
        true
    }

    pub fn resize(&self, _width: u32, _height: u32) -> bool {
        true
    }
}
