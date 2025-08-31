#[path = "window.rs"]
mod window;

use window::{Button, Key};

#[path = "event.rs"]
mod event;

use event::{EventCodes, EventCtx, EventState};

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

pub enum InputError {
    AlreadyInitialized,
    AlreadyShutdown,
    NotInitialized,
}

pub struct InputState {
    keyboard_current: KeyboardState,
    keyboard_previous: KeyboardState,
    mouse_current: MouseState,
    mouse_previous: MouseState,
}

static mut INPUT_STATE: Option<InputState> = None;

impl InputState {
    pub fn initialize() -> Result<(), InputError> {
        unsafe {
            if let Some(ref _a) = INPUT_STATE {
                return Err(InputError::AlreadyInitialized);
            } else {
                INPUT_STATE = Some(InputState {
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
                });
            }
        }
        Ok(())
    }

    pub fn shutdown() -> Result<(), InputError> {
        unsafe {
            if let Some(ref _a) = INPUT_STATE {
                INPUT_STATE = None;
            } else {
                return Err(InputError::AlreadyShutdown);
            }
        }
        Ok(())
    }

    pub fn update(_delta_time: f32) -> Result<(), InputError> {
        unsafe {
            if let Some(ref mut state) = INPUT_STATE {
                state.keyboard_previous = state.keyboard_current;
                state.mouse_previous = state.mouse_current;
            } else {
                return Err(InputError::NotInitialized);
            }
        }

        Ok(())
    }

    pub fn process_key(key: Key, pressed: bool) -> Result<(), InputError> {
        let state = unsafe {
            if let Some(ref mut s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        if state.keyboard_current.keys[key as usize] != pressed as u8 {
            state.keyboard_current.keys[key as usize] = pressed as u8;
            let mut arr = [0u16; 8];
            arr[0] = key as u16;
            let ctx = EventCtx::U16(arr);
            let code = if pressed {
                EventCodes::KeyPressed as usize
            } else {
                EventCodes::KeyReleased as usize
            };
            match EventState::fire_event(code, std::ptr::null(), &ctx) {
                Ok(()) => (),
                Err(_) => return Err(InputError::NotInitialized),
            }
        }

        Ok(())
    }

    pub fn process_button(button: Button, pressed: bool) -> Result<(), InputError> {
        let state = unsafe {
            if let Some(ref mut s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        if state.mouse_current.buttons[button as usize] != pressed as u8 {
            state.mouse_current.buttons[button as usize] = pressed as u8;
            let mut arr = [0u16; 8];
            arr[0] = button as u16;
            let ctx = EventCtx::U16(arr);
            let code = if pressed {
                EventCodes::ButtonPressed as usize
            } else {
                EventCodes::ButtonReleased as usize
            };
            match EventState::fire_event(code, std::ptr::null(), &ctx) {
                Ok(()) => (),
                Err(_) => return Err(InputError::NotInitialized),
            }
        }

        Ok(())
    }

    pub fn process_mouse_move(x: i16, y: i16) -> Result<(), InputError> {
        let state = unsafe {
            if let Some(ref mut s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        if state.mouse_current.pos_x != x || state.mouse_current.pos_y != y {
            state.mouse_current.pos_x = x;
            state.mouse_current.pos_y = y;
            let mut arr = [0u16; 8];
            arr[0] = x as u16;
            arr[1] = y as u16;
            let ctx = EventCtx::U16(arr);
            match EventState::fire_event(EventCodes::MouseMoved as usize, std::ptr::null(), &ctx) {
                Ok(()) => (),
                Err(_) => return Err(InputError::NotInitialized),
            }
        }

        Ok(())
    }

    pub fn process_mouse_wheel(z_delta: i8) -> Result<(), InputError> {
        let mut arr = [0i8; 16];
        arr[0] = z_delta;
        let ctx = EventCtx::I8(arr);
        match EventState::fire_event(EventCodes::MouseWheel as usize, std::ptr::null(), &ctx) {
            Ok(_) => Ok(()),
            Err(_) => return Err(InputError::NotInitialized),
        }
    }

    pub fn is_key_down(key: Key) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        Ok(state.keyboard_current.keys[key as usize] == true as _)
    }

    pub fn is_key_up(key: Key) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        Ok(state.keyboard_current.keys[key as usize] == false as _)
    }

    pub fn was_key_down(key: Key) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        Ok(state.keyboard_previous.keys[key as usize] == true as _)
    }

    pub fn was_key_up(key: Key) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };

        Ok(state.keyboard_previous.keys[key as usize] == false as _)
    }

    pub fn is_button_down(button: Button) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };
        Ok(state.mouse_current.buttons[button as usize] == true as _)
    }

    pub fn is_button_up(button: Button) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };
        Ok(state.mouse_current.buttons[button as usize] == false as _)
    }

    pub fn was_button_down(button: Button) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };
        Ok(state.mouse_previous.buttons[button as usize] == true as _)
    }

    pub fn was_button_up(button: Button) -> Result<bool, InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };
        Ok(state.mouse_previous.buttons[button as usize] == false as _)
    }

    pub fn get_mouse_pos() -> Result<(i32, i32), InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };
        Ok((
            state.mouse_current.pos_x.into(),
            state.mouse_current.pos_y.into(),
        ))
    }

    pub fn get_prev_mouse_pos() -> Result<(i32, i32), InputError> {
        let state = unsafe {
            if let Some(ref s) = INPUT_STATE {
                s
            } else {
                return Err(InputError::NotInitialized);
            }
        };
        Ok((
            state.mouse_previous.pos_x.into(),
            state.mouse_previous.pos_y.into(),
        ))
    }
    
}
