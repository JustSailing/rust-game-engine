mod application;

use std::cell::RefCell;
use std::rc::Rc;

use application::basic::math::vec3::Vec3;
use application::{AppConfig, ApplicationState};

use crate::application::basic::event::{EventCallback, EventCodes, EventCtx};
use crate::application::basic::input::InputState;
use crate::application::basic::window::Key;
use crate::application::renderer::frontend_renderer::Renderer;
use crate::application::renderer::renderer_types::RendererDebugViewMode;
use crate::application::resources::resource_types::Mesh;
use crate::application::systems::camera_system::CameraSystem;
use crate::application::systems::material_system::MaterialSystem;
use crate::application::systems::texture_system::TextureSystem;

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
    texture_system: Rc<RefCell<TextureSystem>>,
    renderer_system: Rc<RefCell<Renderer>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    camera_system: Rc<RefCell<CameraSystem>>,
}

impl<'a> Game<'a> {
    // temporary
    pub fn initialize(&mut self) -> bool {
        self.camera_system
            .borrow_mut()
            .get_mut_default_camera()
            .set_position(&Vec3::new(10.5, 5.0, 9.5));
        true
    }

    pub fn update(
        &mut self,
        delta: f32,
        input_system: &Rc<RefCell<InputState>>,
        renderer: &Rc<RefCell<Renderer>>,
        _meshes: &mut Vec<Mesh>,
    ) -> bool {
        let movement = 8000.0;

        if input_system.borrow().is_key_down(Key::A).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .yaw(1.0 * delta * movement);
        }

        if input_system.borrow().is_key_down(Key::D).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .yaw(-1.0 * delta * movement);
        }
        if input_system.borrow().is_key_down(Key::Up).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .pitch(1.0 * delta * movement);
        }

        if input_system.borrow().is_key_down(Key::Down).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .pitch(-1.0 * delta * movement);
        }

        if input_system.borrow().is_key_down(Key::W).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .move_forward(movement * delta);
        }

        if input_system.borrow().is_key_down(Key::S).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .move_backward(movement * delta);
        }

        if input_system.borrow().is_key_down(Key::Q).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .move_left(movement * delta);
        }

        if input_system.borrow().is_key_down(Key::E).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .move_right(movement * delta);
        }

        if input_system.borrow().is_key_down(Key::Z).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .move_up(movement * delta);
        }

        if input_system.borrow().is_key_down(Key::C).unwrap() {
            self.camera_system
                .borrow_mut()
                .get_mut_default_camera()
                .move_down(movement * delta);
        }

        if input_system.borrow().is_key_down(Key::T).unwrap() {
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

        if input_system.borrow().is_key_down(Key::_1).unwrap() {
            let _ = renderer
                .borrow_mut()
                .set_render_mode(RendererDebugViewMode::Lighting as u32);
        }

        if input_system.borrow().is_key_down(Key::_2).unwrap() {
            let _ = renderer
                .borrow_mut()
                .set_render_mode(RendererDebugViewMode::Normals as u32);
        }

        if input_system.borrow().is_key_down(Key::_0).unwrap() {
            let _ = renderer
                .borrow_mut()
                .set_render_mode(RendererDebugViewMode::Default as u32);
        }

        self.camera_system
            .borrow_mut()
            .get_mut_default_camera()
            .recalculate_view();
        true
    }

    pub fn render(&self, _delta: f32) -> bool {
        true
    }

    pub fn resize(&self, _width: u32, _height: u32) -> bool {
        true
    }
}

impl<'a> EventCallback for Game<'a> {
    fn handle_event(
        &mut self,
        code: usize,
        _sender: *const std::os::raw::c_void,
        _listener: *const std::os::raw::c_void,
        data: &EventCtx,
    ) -> bool {
        match EventCodes::from(code) {
            EventCodes::ApplicationQuit => {
                println!("in game event call back handle event");
                true
            }
            EventCodes::KeyPressed => todo!(),
            EventCodes::KeyReleased => todo!(),
            EventCodes::ButtonPressed => todo!(),
            EventCodes::ButtonReleased => todo!(),
            EventCodes::MouseMoved => todo!(),
            EventCodes::MouseWheel => todo!(),
            EventCodes::WindowResized => {
                let arr = match data {
                    EventCtx::I32(arr) => arr,
                    _ => return false,
                };
                println!(
                    "in game event callback on resize: width {} height {} x {} y {}",
                    arr[0], arr[1], arr[2], arr[3]
                );

                match self.renderer_system.borrow_mut().on_resize(arr[0], arr[1]) {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            EventCodes::SetRenderMode => todo!(),
            EventCodes::Debug0 => todo!(),
            EventCodes::MaxCodes => todo!(),
        }
    }
}
