mod application;

use std::cell::RefCell;
use std::rc::Rc;

use application::basic::{math::consts::deg_to_rad, math::matrix4::Matrix4, math::vec3::Vec3};
use application::{AppConfig, ApplicationState};

use crate::application::basic::event::{EventCallback, EventCodes, EventCtx};
use crate::application::basic::input::InputState;
use crate::application::basic::window::Key;
use crate::application::renderer::renderer_types::{Renderer, RendererDebugViewMode};
use crate::application::resources::resource_types::Geometry;
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
    view: Matrix4,
    camera_position: Vec3,
    camera_euler: Vec3,
    view_dirty: bool,
}

#[derive(Clone)]
pub struct Game<'a> {
    config: AppConfig,
    state: GameState,
    texture_system: Rc<RefCell<TextureSystem>>,
    renderer_system: Rc<RefCell<Renderer>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    test_geometry: Rc<RefCell<Geometry>>,
}

impl<'a> Game<'a> {
    // temporary
    pub fn initialize(&mut self, geometry: Rc<RefCell<Geometry>>) -> bool {
        self.state.camera_position = Vec3::new(0.0, 0.0, 30.0);
        self.state.camera_euler = Vec3::new_zeroes();
        self.state.view = Matrix4::translation(&self.state.camera_position);
        self.state.view = Matrix4::inverse(&self.state.view);
        self.state.view_dirty = true;
        self.test_geometry = geometry;
        return true;
    }

    pub fn update(
        &mut self,
        delta: f32,
        input_system: &Rc<RefCell<InputState>>,
        renderer: &Rc<RefCell<Renderer>>,
    ) -> bool {
        let movement = 15000.0;

        if input_system.borrow().is_key_down(Key::A).unwrap() {
            self.camera_yaw(1.0 * delta * movement);
        }

        if input_system.borrow().is_key_down(Key::D).unwrap() {
            self.camera_yaw(-1.0 * delta * movement);
        }
        if input_system.borrow().is_key_down(Key::Up).unwrap() {
            self.camera_pitch(1.0 * delta * movement);
        }

        if input_system.borrow().is_key_down(Key::Down).unwrap() {
            self.camera_pitch(-1.0 * delta * movement);
        }

        let temp_move_speed = 150000.0;
        let mut velocity = Vec3::new_ones();
        if input_system.borrow().is_key_down(Key::W).unwrap() {
            let forward: Vec3 = self.state.view.forward();
            velocity = velocity + forward.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.borrow().is_key_down(Key::S).unwrap() {
            let backward = self.state.view.backward();
            velocity = velocity + backward.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.borrow().is_key_down(Key::Q).unwrap() {
            let left = self.state.view.left();
            velocity = velocity + left.mul_scalar(temp_move_speed);
        }

        if input_system.borrow().is_key_down(Key::E).unwrap() {
            let right = self.state.view.right();
            velocity = velocity + right.mul_scalar(temp_move_speed);
        }

        if input_system.borrow().is_key_down(Key::Z).unwrap() {
            let up: Vec3 = self.state.view.up();
            velocity = velocity + up.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.borrow().is_key_down(Key::C).unwrap() {
            let down = self.state.view.down();
            velocity = velocity + down.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.borrow().is_key_down(Key::Space).unwrap() {
            velocity.data[1] += 1.0;
        }

        if input_system.borrow().is_key_down(Key::Space).unwrap() {
            velocity.data[1] -= 1.0;
        }

        if input_system.borrow().is_key_down(Key::E).unwrap() {
            let right = self.state.view.right();
            velocity = velocity + right.mul_scalar(temp_move_speed);
        }

        //velocity.normalize();
        self.state.camera_position.data[0] += velocity.data[0] * delta;
        self.state.camera_position.data[0] += velocity.data[0] * delta;
        self.state.camera_position.data[0] += velocity.data[0] * delta;
        self.state.view_dirty = true;

        if input_system.borrow().is_key_down(Key::T).unwrap() {
            let names = ["brick-wall", "door", "stone-wall", "tile", "cube"];
            let spec_names = [
                "brick-wall_spec",
                "door_spec",
                "stone-wall_spec",
                "tile_spec",
                "cube_spec",
            ];
            let norm_names = [
                "brick-wall_norm",
                "door_norm",
                "stone-wall_norm",
                "tile_norm",
                "cube_norm",
            ];
            static mut CHOICE: usize = 4;
            let old_name = unsafe { names[CHOICE] };
            let old_spec_name = unsafe { spec_names[CHOICE] };
            let old_norm_name = unsafe { norm_names[CHOICE] };
            unsafe {
                CHOICE += 1;
                CHOICE %= 5;
            }

            let tex = match self.texture_system.borrow_mut().acquire(
                unsafe { names[CHOICE].to_string() },
                if unsafe { names[CHOICE] == "cube" } {
                    "png"
                } else {
                    "jpg"
                },
                true,
            ) {
                Ok(t) => t,
                Err(_) => return false,
            };

            match self.texture_system.borrow_mut().release(old_name) {
                Ok(_) => {}
                Err(_) => return false,
            }

            self.test_geometry
                .borrow_mut()
                .material
                .borrow_mut()
                .diffuse_map
                .texture = tex;

            let spec = match self.texture_system.borrow_mut().acquire(
                unsafe { spec_names[CHOICE].to_string() },
                if unsafe { spec_names[CHOICE] == "cube_spec" } {
                    "png"
                } else {
                    "jpg"
                },
                true,
            ) {
                Ok(t) => t,
                Err(_) => return false,
            };

            match self.texture_system.borrow_mut().release(old_spec_name) {
                Ok(_) => {}
                Err(_) => return false,
            }

            self.test_geometry
                .borrow_mut()
                .material
                .borrow_mut()
                .specular_map
                .texture = spec;

            let norm = match self.texture_system.borrow_mut().acquire(
                unsafe { norm_names[CHOICE].to_string() },
                if unsafe { norm_names[CHOICE] == "cube_norm" } {
                    "png"
                } else {
                    "jpg"
                },
                true,
            ) {
                Ok(t) => t,
                Err(_) => return false,
            };

            match self.texture_system.borrow_mut().release(old_norm_name) {
                Ok(_) => {}
                Err(_) => return false,
            }

            self.test_geometry
                .borrow_mut()
                .material
                .borrow_mut()
                .normal_map
                .texture = norm;
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

        self.state.view_dirty = true;
        self.recalculate_view();

        match renderer
            .borrow_mut()
            .set_view(self.state.view, self.state.camera_position)
        {
            Ok(_) => return true,
            Err(_) => return false,
        }
    }

    pub fn render(&self, _delta: f32) -> bool {
        return true;
    }

    pub fn resize(&self, _width: u32, _height: u32) -> bool {
        return true;
    }

    fn recalculate_view(&mut self) {
        if self.state.view_dirty {
            let rotation = Matrix4::euler_xyz(
                self.state.camera_euler.data[0],
                self.state.camera_euler.data[1],
                self.state.camera_euler.data[2],
            );

            let translation = Matrix4::translation(&self.state.camera_position);
            self.state.view = translation * rotation;
            self.state.view = Matrix4::inverse(&self.state.view);
            self.state.view_dirty = false;
        }
    }

    fn camera_yaw(&mut self, amount: f32) {
        self.state.camera_euler.data[1] += amount; // y axis
        self.state.view_dirty = true;
    }

    fn camera_pitch(&mut self, amount: f32) {
        self.state.camera_euler.data[0] += amount;
        let limit = deg_to_rad(89.0);
        self.state.camera_euler.data[0] = self.state.camera_euler.data[0].min(limit).max(-limit);
        self.state.view_dirty = true;
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
                println!("in event call back handle event");
                return true;
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
