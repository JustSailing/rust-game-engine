mod application;

use std::cell::RefCell;
use std::ptr;
use std::rc::Rc;

use application::basic::{math::consts::deg_to_rad, math::matrix4::Matrix4, math::vec3::Vec3};
use application::{AppConfig, ApplicationState};

use crate::application::basic::event::{EventCallback, EventCodes, EventCtx, EventSystem};
use crate::application::basic::input::InputState;
use crate::application::basic::window::Key;
use crate::application::renderer::renderer_types::Renderer;
use crate::application::resources::resource_types::Geometry;
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
pub struct Game {
    config: AppConfig,
    state: GameState,
    texture_system: Rc<RefCell<TextureSystem>>,
    renderer_system: Rc<RefCell<Renderer>>,
    test_geometry: Rc<RefCell<Geometry>>,
}

impl Game {
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
        input_system: &InputState,
        renderer: &mut Renderer,
        event_system: &mut EventSystem,
    ) -> bool {
        let movement = 15000.0;

        if input_system.is_key_down(Key::A).unwrap() {
            self.camera_yaw(1.0 * delta * movement);
        }

        if input_system.is_key_down(Key::D).unwrap() {
            self.camera_yaw(-1.0 * delta * movement);
        }
        if input_system.is_key_down(Key::Up).unwrap() {
            self.camera_pitch(1.0 * delta * movement);
        }

        if input_system.is_key_down(Key::Down).unwrap() {
            self.camera_pitch(-1.0 * delta * movement);
        }

        let temp_move_speed = 150000.0;
        let mut velocity = Vec3::new_ones();
        if input_system.is_key_down(Key::W).unwrap() {
            let forward: Vec3 = self.state.view.forward();
            velocity = velocity + forward.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.is_key_down(Key::S).unwrap() {
            let backward = self.state.view.backward();
            velocity = velocity + backward.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.is_key_down(Key::Q).unwrap() {
            let left = self.state.view.left();
            velocity = velocity + left.mul_scalar(temp_move_speed);
        }

        if input_system.is_key_down(Key::E).unwrap() {
            let right = self.state.view.right();
            velocity = velocity + right.mul_scalar(temp_move_speed);
        }

        if input_system.is_key_down(Key::Z).unwrap() {
            let up: Vec3 = self.state.view.up();
            velocity = velocity + up.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.is_key_down(Key::C).unwrap() {
            let down = self.state.view.down();
            velocity = velocity + down.mul_scalar(temp_move_speed * 10.0);
        }

        if input_system.is_key_down(Key::Space).unwrap() {
            velocity.data[1] += 1.0;
        }

        if input_system.is_key_down(Key::Space).unwrap() {
            velocity.data[1] -= 1.0;
        }

        if input_system.is_key_down(Key::E).unwrap() {
            let right = self.state.view.right();
            velocity = velocity + right.mul_scalar(temp_move_speed);
        }

        //velocity.normalize();
        self.state.camera_position.data[0] += velocity.data[0] * delta;
        self.state.camera_position.data[0] += velocity.data[0] * delta;
        self.state.camera_position.data[0] += velocity.data[0] * delta;
        self.state.view_dirty = true;

        if input_system.is_key_down(Key::T).unwrap() {
            let ctx: EventCtx = EventCtx::I32([0; 4]);
            let _ = event_system.fire_event(EventCodes::Debug0 as usize, ptr::null(), &ctx);
        }
        self.state.view_dirty = true;
        self.recalculate_view();

        match renderer.set_view(self.state.view, self.state.camera_position) {
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

impl EventCallback for Game {
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
                //self.is_running = false;
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
            EventCodes::Debug0 => {
                let names = ["brick-wall", "door", "stone-wall", "tile"];
                static mut CHOICE: usize = 3;
                let old_name = unsafe { names[CHOICE] };
                unsafe {
                    CHOICE += 1;
                    CHOICE %= 4;
                }

                self.test_geometry
                    .borrow_mut()
                    .material
                    .borrow_mut()
                    .diffuse_map
                    .texture = match self.texture_system.borrow_mut().acquire(
                    unsafe { names[CHOICE].to_string() },
                    "jpg",
                    true,
                ) {
                    Ok(t) => t,
                    Err(_) => return false,
                };

                match self.texture_system.borrow_mut().release(old_name) {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            EventCodes::MaxCodes => todo!(),
        }
    }
}
