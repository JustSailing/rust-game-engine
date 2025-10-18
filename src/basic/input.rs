use std::cell::RefCell;
use std::rc::Rc;

use super::event::{EventCodes, EventCtx, EventSystem};
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

pub struct InputState<'a> {
    keyboard_current: KeyboardState,
    keyboard_previous: KeyboardState,
    mouse_current: MouseState,
    mouse_previous: MouseState,
    event_system: Rc<RefCell<EventSystem<'a>>>,
}

impl<'a> InputState<'a> {
    pub fn initialize(event_system: Rc<RefCell<EventSystem<'a>>>) -> Result<Self> {
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
            event_system,
        })
    }

    pub fn update(&mut self, _delta_time: f32) -> Result<()> {
        self.keyboard_previous = self.keyboard_current;
        self.mouse_previous = self.mouse_current;

        Ok(())
    }

    pub fn process_key(&mut self, key: Key, pressed: bool) -> Result<()> {
        if self.keyboard_current.keys[key as usize] != pressed as u8 {
            self.keyboard_current.keys[key as usize] = pressed as u8;
            let mut arr = [0u16; 8];
            arr[0] = key as u16;
            let ctx = EventCtx::U16(arr);
            let code = if pressed {
                EventCodes::KeyPressed as usize
            } else {
                EventCodes::KeyReleased as usize
            };
            match self
                .event_system
                .borrow_mut()
                .fire_event(code, std::ptr::null(), &ctx)
            {
                Ok(()) => (),
                Err(_) => return Err(InputSysError::NotInitialized),
            }
        }

        Ok(())
    }

    pub fn process_button(&mut self, button: Button, pressed: bool) -> Result<()> {
        if self.mouse_current.buttons[button as usize] != pressed as u8 {
            self.mouse_current.buttons[button as usize] = pressed as u8;
            let mut arr = [0u16; 8];
            arr[0] = button as u16;
            let ctx = EventCtx::U16(arr);
            let code = if pressed {
                EventCodes::ButtonPressed as usize
            } else {
                EventCodes::ButtonReleased as usize
            };
            match self
                .event_system
                .borrow_mut()
                .fire_event(code, std::ptr::null(), &ctx)
            {
                Ok(()) => (),
                Err(_) => return Err(InputSysError::NotInitialized),
            }
        }

        Ok(())
    }

    pub fn process_mouse_move(&mut self, x: i16, y: i16) -> Result<()> {
        if self.mouse_current.pos_x != x || self.mouse_current.pos_y != y {
            self.mouse_current.pos_x = x;
            self.mouse_current.pos_y = y;
            let mut arr = [0u16; 8];
            arr[0] = x as u16;
            arr[1] = y as u16;
            let ctx = EventCtx::U16(arr);
            match self.event_system.borrow_mut().fire_event(
                EventCodes::MouseMoved as usize,
                std::ptr::null(),
                &ctx,
            ) {
                Ok(()) => (),
                Err(_) => return Err(InputSysError::NotInitialized),
            }
        }

        Ok(())
    }

    pub fn process_mouse_wheel(&self, z_delta: i8) -> Result<()> {
        let mut arr = [0i8; 16];
        arr[0] = z_delta;
        let ctx = EventCtx::I8(arr);
        match self.event_system.borrow_mut().fire_event(
            EventCodes::MouseWheel as usize,
            std::ptr::null(),
            &ctx,
        ) {
            Ok(_) => Ok(()),
            Err(_) => return Err(InputSysError::NotInitialized),
        }
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
