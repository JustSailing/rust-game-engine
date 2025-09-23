mod application;
use application::{AppConfig, AppError, ApplicationState};

fn main() -> Result<(), AppError> {
    println!("Hello, world!");
    let init = ApplicationState::create(&AppConfig {
        start_pos_x: 0,
        start_pos_y: 0,
        start_width: 1280,
        start_height: 720,
        name: "Hello William",
    });
    println!("{:?}", init);
    let r = ApplicationState::run();
    println!("{:?}", r);
    let s = ApplicationState::shutdown();
    println!("{:?}", s);
    Ok(())
}
