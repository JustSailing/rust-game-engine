mod application;
use std::ptr;

use application::basic::{
    input::InputState, math::consts::deg_to_rad, math::matrix4::Matrix4, math::vec3::Vec3,
    window::Key,
};
use application::{AppConfig, ApplicationState};

use crate::application::basic::event::{EventCodes, EventCtx};
use crate::application::renderer::renderer_types::Renderer;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Hello, world!");
    let app_config = AppConfig {
        start_pos_x: 0,
        start_pos_y: 0,
        start_width: 1280,
        start_height: 720,
        name: "Hello William",
    };
    let mut game = Game {
        config: app_config,
        state: GameState {
            delta_time: 0.0,
            view: Matrix4::new_zeros(),
            camera_position: Vec3::new_zeroes(),
            camera_euler: Vec3::new_zeroes(),
            view_dirty: false,
        },
        initialize: game_initialize,
        update: game_update,
        render: game_render,
        on_resize: game_on_resize,
    };
    let init = ApplicationState::create(&mut game);
    println!("{:?}", init);
    let r = ApplicationState::run();
    println!("{:?}", r);
    let s = ApplicationState::shutdown();
    println!("{:?}", s);
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

#[derive(Clone, Copy)]
pub struct Game {
    config: AppConfig,
    state: GameState,
    initialize: fn(game: &mut Game) -> bool,
    update: fn(game: &mut Game, delta: f32) -> bool,
    render: fn(game: &Game, delta: f32) -> bool,
    on_resize: fn(game: &Game, width: u32, height: u32) -> bool,
}

pub fn game_initialize(game: &mut Game) -> bool {
    game.state.camera_position = Vec3::new(0.0, 0.0, -30.0);
    game.state.camera_euler = Vec3::new_zeroes();
    game.state.view = Matrix4::translation(&game.state.camera_position);
    return true;
}

pub fn game_update(game: &mut Game, delta: f32) -> bool {
    let movement = 500.0;

    if InputState::is_key_down(Key::A).unwrap() {
        camera_yaw(&mut game.state, 1.0 * delta * movement);
    }

    if InputState::is_key_down(Key::D).unwrap() {
        camera_yaw(&mut game.state, -1.0 * delta * movement);
    }
    if InputState::is_key_down(Key::W).unwrap() {
        camera_pitch(&mut game.state, 1.0 * delta * movement);
    }

    if InputState::is_key_down(Key::S).unwrap() {
        camera_pitch(&mut game.state, -1.0 * delta * movement);
    }

    if InputState::is_key_down(Key::T).unwrap() {
        let ctx: EventCtx = EventCtx::I32([0; 4]);
        let _ = application::on_event_debug(
            EventCodes::Debug0 as usize,
            ptr::null(),
            ptr::null(),
            &ctx,
        );
    }

    recalculate_view(&mut game.state);

    match Renderer::set_view(game.state.view) {
        Ok(_) => return true,
        Err(_) => return false,
    }
}

pub fn game_render(_game: &Game, _delta: f32) -> bool {
    return true;
}

pub fn game_on_resize(_game: &Game, _width: u32, _height: u32) -> bool {
    return true;
}

fn recalculate_view(state: &mut GameState) {
    if state.view_dirty {
        let rotation = Matrix4::euler_xyz(
            state.camera_euler.data[0],
            state.camera_euler.data[1],
            state.camera_euler.data[2],
        );

        let translation = Matrix4::translation(&state.camera_position);
        state.view = rotation * translation;
        state.view_dirty = false;
    }
}

fn camera_yaw(state: &mut GameState, amount: f32) {
    state.camera_euler.data[1] += amount; // y axis
    state.view_dirty = true;
}

fn camera_pitch(state: &mut GameState, amount: f32) {
    state.camera_euler.data[0] += amount;
    let limit = deg_to_rad(89.0);
    state.camera_euler.data[0] = state.camera_euler.data[0].min(limit).max(-limit);
    state.view_dirty = true;
}
