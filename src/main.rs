mod application;

use application::*;

fn main() -> Result<(), AppError> {
    println!("Hello, world!");
    let _ = application::ApplicationState::create(&AppConfig {
        start_pos_x: 0,
        start_pos_y: 0,
        start_width: 1000,
        start_height: 800,
        name: "Hello William",
    });
    let _ = application::ApplicationState::run();
    Ok(())
}
