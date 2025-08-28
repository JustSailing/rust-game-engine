mod basic;

use basic::window::{Display, Window, X11Error, Event};

fn main() -> Result<(), X11Error>{
    println!("Hello, world!");
    let display = Display::open()?;
    let window = Window::create(&display, 900, 500)?;
    window.set_title("Hello World");
    window.show();
    loop {
        if let Some(event) = window.get_event() {
            match event {
                Event::Key { key } => continue,
                Event::Button { button } => continue,
                Event::MousePos { x, y } => continue,
                Event::ConfigureNotify { x, y, width, height } => continue,
                Event::CloseWindow => {return Ok(());},
            }
        } else {
            continue;
        }
    }

}
