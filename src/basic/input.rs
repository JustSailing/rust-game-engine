use super::event::{EventCodes, EventCtx};
use super::window::{Button, Key};
use thiserror::Error;

type Result<T> = std::result::Result<T, InputSysError>;

#[derive(Clone, Copy)]
struct KeyboardState {
    keys: [u8; 256],
}

#[derive(Clone, Copy)]
struct MouseState {
    pos_x: i16,
    pos_y: i16,
    buttons: [u8; Button::MaxButtons as usize],
}

#[derive(Error, Debug)]
pub enum InputSysError {
    #[error("input system error: already initialized {} {}", file!(), line!())]
    AlreadyInitialized,
    #[error("input system error: already shutdown {} {}", file!(), line!())]
    AlreadyShutdown,
    #[error("input system error: not initialized {} {}", file!(), line!())]
    NotInitialized,
}

pub struct InputState {
    keyboard_current: KeyboardState,
    keyboard_previous: KeyboardState,
    mouse_current: MouseState,
    mouse_previous: MouseState,
}

impl InputState {
    pub fn initialize() -> Result<Self> {
        Ok(InputState {
            keyboard_current: KeyboardState { keys: [0; 256] },
            keyboard_previous: KeyboardState { keys: [0; 256] },
            mouse_current: MouseState {
                pos_x: 0,
                pos_y: 0,
                buttons: [0; Button::MaxButtons as usize],
            },
            mouse_previous: MouseState {
                pos_x: 0,
                pos_y: 0,
                buttons: [0; Button::MaxButtons as usize],
            },
        })
    }

    pub fn update(&mut self, _delta_time: f32) -> Result<()> {
        self.keyboard_previous = self.keyboard_current;
        self.mouse_previous = self.mouse_current;

        Ok(())
    }

    pub fn handle_event(&mut self, code: usize, data: &EventCtx) -> Result<()> {
        match EventCodes::from(code) {
            EventCodes::KeyPressed => match data {
                EventCtx::U16(d) => {
                    self.process_key(d[0], true)?;
                }
                _ => {}
            },
            EventCodes::KeyReleased => match data {
                EventCtx::U16(d) => {
                    self.process_key(d[0], false)?;
                }
                _ => {}
            },
            EventCodes::ButtonPressed => match data {
                EventCtx::U16(d) => {
                    self.process_button(d[0], true)?;
                }
                _ => {}
            },
            EventCodes::ButtonReleased => match data {
                EventCtx::U16(d) => {
                    self.process_button(d[0], false)?;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn process_key(&mut self, key: u16, pressed: bool) -> Result<()> {
        if self.keyboard_current.keys[key as usize] != pressed as u8 {
            self.keyboard_current.keys[key as usize] = pressed as u8;
        }
        Ok(())
    }

    pub fn process_button(&mut self, button: u16, pressed: bool) -> Result<()> {
        if self.mouse_current.buttons[button as usize] != pressed as u8 {
            self.mouse_current.buttons[button as usize] = pressed as u8;
        }

        Ok(())
    }

    pub fn process_mouse_move(&mut self, x: i16, y: i16) -> Result<()> {
        if self.mouse_current.pos_x != x || self.mouse_current.pos_y != y {
            self.mouse_current.pos_x = x;
            self.mouse_current.pos_y = y;
        }

        Ok(())
    }

    pub fn process_mouse_wheel(&self, _z_delta: i8) -> Result<()> {
        // TODO: Add mouse wheel to input state
        Ok(())
    }

    pub fn is_key_down(&self, key: Key) -> Result<bool> {
        Ok(self.keyboard_current.keys[key as usize] == true as _)
    }

    pub fn is_key_up(&self, key: Key) -> Result<bool> {
        Ok(self.keyboard_current.keys[key as usize] == false as _)
    }

    pub fn was_key_down(&self, key: Key) -> Result<bool> {
        Ok(self.keyboard_previous.keys[key as usize] == true as _)
    }

    pub fn was_key_up(&self, key: Key) -> Result<bool> {
        Ok(self.keyboard_previous.keys[key as usize] == false as _)
    }

    pub fn is_button_down(&self, button: Button) -> Result<bool> {
        Ok(self.mouse_current.buttons[button as usize] == true as _)
    }

    pub fn is_button_up(&self, button: Button) -> Result<bool> {
        Ok(self.mouse_current.buttons[button as usize] == false as _)
    }

    pub fn was_button_down(&self, button: Button) -> Result<bool> {
        Ok(self.mouse_previous.buttons[button as usize] == true as _)
    }

    pub fn was_button_up(&self, button: Button) -> Result<bool> {
        Ok(self.mouse_previous.buttons[button as usize] == false as _)
    }

    pub fn get_mouse_pos(&self) -> Result<(i32, i32)> {
        Ok((
            self.mouse_current.pos_x as i32,
            self.mouse_current.pos_y as i32,
        ))
    }

    pub fn get_prev_mouse_pos(&self) -> Result<(i32, i32)> {
        Ok((
            self.mouse_previous.pos_x as i32,
            self.mouse_previous.pos_y as i32,
        ))
    }
}
