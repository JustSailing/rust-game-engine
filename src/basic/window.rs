use std::cell::RefCell;
use std::ffi::CString;
use std::mem;
use std::ptr;

use std::os::raw::*;
use std::rc::Rc;
use x11::keysym::*;
use x11::xlib::Display as Display_;
use x11::xlib::Window as Window_;
use x11::xlib::*;

use crate::application::basic::event::{EventCodes, EventCtx, EventSysError, EventSystem};

use super::input::{InputState, InputSysError};

use thiserror::Error;

type Result<T> = std::result::Result<T, WindowError>;

#[derive(Error, Debug)]
pub enum WindowError {
    #[error("window error: {issue} {} {}", file!(), line!())]
    OperationFailed { issue: String },
    #[error("window error: from input system {source} {} {}", file!(), line!())]
    InputStateError {
        #[from]
        source: InputSysError,
    },
    #[error("window error: from event system {source} {} {}", file!(), line!())]
    EventStateError {
        #[from]
        source: EventSysError,
    },
}

pub struct Display {
    pub raw: *mut Display_,
}

impl Display {
    pub fn open() -> Result<Self> {
        let display = unsafe { XOpenDisplay(ptr::null_mut()) };
        if display.is_null() {
            return Err(WindowError::OperationFailed {
                issue: "failed to open display".to_string(),
            });
        }
        Ok(Display { raw: display })
    }

    pub fn sync(&self) {
        unsafe {
            XSync(self.raw, False as _);
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(self.raw) };
    }
}

pub struct Window<'a> {
    pub display: Display,
    pub window_id: Window_,
    wm_protocols: Atom,
    wm_delete: Atom,
    pub width: u32,
    pub height: u32,
    input_system: Rc<RefCell<InputState<'a>>>,
    event_system: Rc<RefCell<EventSystem<'a>>>,
}

impl<'a> Window<'a> {
    pub fn create(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        input_system: Rc<RefCell<InputState<'a>>>,
        event_system: Rc<RefCell<EventSystem<'a>>>,
    ) -> Result<Self> {
        let display = match Display::open() {
            Ok(d) => d,
            Err(_) => {
                return Err(WindowError::OperationFailed {
                    issue: "Could not open display".to_string(),
                });
            }
        };
        let screen_num = unsafe { XDefaultScreen(display.raw) };
        let root_win_id = unsafe { XRootWindow(display.raw, screen_num) };
        let window_id = unsafe {
            XCreateSimpleWindow(
                display.raw,
                root_win_id,
                x,
                y,
                width as u32,
                height as u32,
                1,
                XWhitePixel(display.raw, screen_num),
                XBlackPixel(display.raw, screen_num),
            )
        };
        if window_id == 0 {
            return Err(WindowError::OperationFailed {
                issue: "failed to create simple window".to_string(),
            });
        }
        let wm_protocols;
        let wm_delete_window;
        unsafe {
            wm_protocols = XInternAtom(
                display.raw,
                b"WM_PROTOCOLS\0".as_ptr() as *const i8,
                False as _,
            );
            wm_delete_window = XInternAtom(
                display.raw,
                b"WM_DELETE_WINDOW\0".as_ptr() as *const i8,
                False as _,
            );
            let mut protocols = [wm_delete_window];
            XSetWMProtocols(
                display.raw,
                window_id,
                protocols.as_mut_ptr(),
                protocols.len() as _,
            );
            XSelectInput(
                display.raw,
                window_id,
                c_long::from(
                    ExposureMask
                        | KeyPressMask
                        | ButtonPressMask
                        | KeyReleaseMask
                        | ButtonReleaseMask
                        | StructureNotifyMask
                        | EnterWindowMask
                        | LeaveWindowMask
                        | PointerMotionMask
                        | Button1MotionMask
                        | Button2MotionMask
                        | Button3MotionMask
                        | Button4MotionMask
                        | Button5MotionMask
                        | StructureNotifyMask
                        | KeymapStateMask,
                ),
            );
        };
        let mut enable = 0;
        unsafe {
            XkbSetDetectableAutoRepeat(display.raw, 1, &mut enable);
        };

        display.sync();

        Ok(Window {
            display,
            window_id,
            wm_protocols,
            wm_delete: wm_delete_window,
            width: width as u32,
            height: height as u32,
            input_system,
            event_system,
        })
    }

    pub fn set_title(&self, title: &str) {
        let title_str = CString::new(title).unwrap();
        unsafe {
            XStoreName(
                self.display.raw,
                self.window_id,
                title_str.as_ptr() as *const i8,
            );
        }
    }

    pub fn show(&self) {
        unsafe {
            XMapWindow(self.display.raw, self.window_id);
        }
    }

    pub fn get_event(&self) -> Result<bool> {
        let mut event: XEvent = unsafe { mem::zeroed() };
        let num_of_events = unsafe { XPending(self.display.raw) };
        if num_of_events == 0 {
            return Ok(true);
        } else {
            unsafe {
                XNextEvent(self.display.raw, &mut event);
                #[allow(non_upper_case_globals)]
                match event.type_ {
                    ClientMessage => {
                        if event.client_message.message_type as Atom == self.wm_protocols
                            && event.client_message.data.as_longs()[0] as Atom == self.wm_delete
                        {
                            let ctx = EventCtx::None;
                            self.event_system.borrow_mut().fire_event(
                                EventCodes::ApplicationQuit as usize,
                                ptr::null(),
                                &ctx,
                            )?;
                            return Ok(false);
                        }
                    }
                    ConfigureNotify => {
                        let ctx = EventCtx::I32([
                            event.configure.width,
                            event.configure.height,
                            event.configure.x,
                            event.configure.y,
                        ]);
                        self.event_system.borrow_mut().fire_event(
                            EventCodes::WindowResized as usize,
                            ptr::null(),
                            &ctx,
                        )?;
                    }
                    KeyPress => {
                        let key_sym =
                            XkbKeycodeToKeysym(self.display.raw, event.key.keycode as u8, 0, 0);
                        let key = Window::keysym_to_key(key_sym);
                        println!("key press {:?}", key);
                        self.input_system.borrow_mut().process_key(key, true)?;
                    }
                    KeyRelease => {
                        let key_sym =
                            XkbKeycodeToKeysym(self.display.raw, event.key.keycode as u8, 0, 0);
                        let key = Window::keysym_to_key(key_sym);
                        println!("key release {:?}", key);
                        self.input_system.borrow_mut().process_key(key, false)?;
                    }
                    ButtonPress => {
                        let mut button = Button::MaxButtons;
                        match event.button.button {
                            Button1 => button = Button::Left,
                            Button2 => button = Button::Middle,
                            Button3 => button = Button::Right,
                            _ => {}
                        };

                        println!("button press {:?}", button);
                        self.input_system
                            .borrow_mut()
                            .process_button(button, true)?;
                    }
                    ButtonRelease => {
                        let mut button = Button::MaxButtons;
                        match event.button.button {
                            Button1 => button = Button::Left,
                            Button2 => button = Button::Middle,
                            Button3 => button = Button::Right,
                            _ => {}
                        };

                        println!("button release {:?}", button);
                        self.input_system
                            .borrow_mut()
                            .process_button(button, true)?;
                    }

                    _ => {} //return Ok(false),
                };
            };
        }
        Ok(true)
    }

    pub fn keysym_to_key(key_sym: u64) -> Key {
        #[allow(non_upper_case_globals)]
        match key_sym as u32 {
            XK_BackSpace => Key::Backspace,
            XK_Return => Key::Enter,
            XK_Tab => Key::Tab,
            XK_Pause => Key::Pause,
            XK_Caps_Lock => Key::Capital,
            XK_Escape => Key::Esc,
            XK_Mode_switch => Key::ModeChange,

            XK_space => Key::Space,
            XK_Prior => Key::Prior,
            XK_Next => Key::Next,
            XK_End => Key::End,
            XK_Home => Key::Home,
            XK_Left => Key::Left,
            XK_Up => Key::Up,
            XK_Right => Key::Right,
            XK_Down => Key::Down,
            XK_Select => Key::Select,
            XK_Print => Key::Print,
            XK_Execute => Key::Execute,
            XK_Insert => Key::Insert,
            XK_Delete => Key::Delete,
            XK_Help => Key::Help,

            XK_KP_0 => Key::NumPad0,
            XK_KP_1 => Key::NumPad1,
            XK_KP_2 => Key::NumPad2,
            XK_KP_3 => Key::NumPad3,
            XK_KP_4 => Key::NumPad4,
            XK_KP_5 => Key::NumPad5,
            XK_KP_6 => Key::NumPad6,
            XK_KP_7 => Key::NumPad7,
            XK_KP_8 => Key::NumPad8,
            XK_KP_9 => Key::NumPad9,
            XK_multiply => Key::Multiply,
            XK_KP_Add => Key::Add,
            XK_KP_Separator => Key::Separator,
            XK_KP_Subtract => Key::Subtract,
            XK_KP_Decimal => Key::Decimal,
            XK_KP_Divide => Key::Divide,

            XK_F1 => Key::F1,
            XK_F2 => Key::F2,
            XK_F3 => Key::F3,
            XK_F4 => Key::F4,
            XK_F5 => Key::F5,
            XK_F6 => Key::F6,
            XK_F7 => Key::F7,
            XK_F8 => Key::F8,
            XK_F9 => Key::F9,
            XK_F10 => Key::F10,
            XK_F11 => Key::F11,
            XK_F12 => Key::F12,
            XK_F13 => Key::F13,
            XK_F14 => Key::F14,
            XK_F15 => Key::F15,
            XK_F16 => Key::F16,
            XK_F17 => Key::F17,
            XK_F18 => Key::F18,
            XK_F19 => Key::F19,
            XK_F20 => Key::F20,
            XK_F21 => Key::F21,
            XK_F22 => Key::F22,
            XK_F23 => Key::F23,
            XK_F24 => Key::F24,

            XK_Num_Lock => Key::NumLock,
            XK_Scroll_Lock => Key::Scroll,

            XK_KP_Equal => Key::NumPadEqual,

            XK_Shift_L => Key::LShift,
            XK_Shift_R => Key::RShift,
            XK_Control_L => Key::LCtrl,
            XK_Control_R => Key::RCtrl,

            XK_semicolon => Key::SemiColon,
            XK_plus => Key::Plus,
            XK_comma => Key::Comma,
            XK_minus => Key::Minus,
            XK_period => Key::Period,
            XK_slash => Key::Slash,
            XK_grave => Key::Grave,

            XK_0 => Key::_0,
            XK_1 => Key::_1,
            XK_2 => Key::_2,
            XK_3 => Key::_3,
            XK_4 => Key::_4,
            XK_5 => Key::_5,
            XK_6 => Key::_6,
            XK_7 => Key::_7,
            XK_8 => Key::_8,
            XK_9 => Key::_9,

            XK_a | XK_A => Key::A,
            XK_b | XK_B => Key::B,
            XK_c | XK_C => Key::C,
            XK_d | XK_D => Key::D,
            XK_e | XK_E => Key::E,
            XK_f | XK_F => Key::F,
            XK_g | XK_G => Key::G,
            XK_h | XK_H => Key::H,
            XK_i | XK_I => Key::I,
            XK_j | XK_J => Key::J,
            XK_k | XK_K => Key::K,
            XK_l | XK_L => Key::L,
            XK_m | XK_M => Key::M,
            XK_n | XK_N => Key::N,
            XK_o | XK_O => Key::O,
            XK_p | XK_P => Key::P,
            XK_q | XK_Q => Key::Q,
            XK_r | XK_R => Key::R,
            XK_s | XK_S => Key::S,
            XK_t | XK_T => Key::T,
            XK_u | XK_U => Key::U,
            XK_v | XK_V => Key::V,
            XK_w | XK_W => Key::W,
            XK_x | XK_X => Key::X,
            XK_y | XK_Y => Key::Y,
            XK_z | XK_Z => Key::Z,
            _ => Key::Unknown,
        }
    }
}

impl<'a> Drop for Window<'a> {
    fn drop(&mut self) {
        unsafe { XDestroyWindow(self.display.raw, self.window_id) };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Unknown = 0x00,
    Backspace = 0x08,
    Enter = 0x0D,
    Tab = 0x09,
    Shift = 0x10,
    Ctrl = 0x11,
    //
    Pause = 0x13,
    Capital = 0x14,
    //
    Esc = 0x1B,
    //
    Convert = 0x1C,
    NonConvert = 0x1D,
    Accept = 0x1E,
    ModeChange = 0x1F,
    //
    Space = 0x20,
    Prior = 0x21,
    Next = 0x22,
    End = 0x23,
    Home = 0x24,
    Left = 0x25,
    Up = 0x26,
    Right = 0x27,
    Down = 0x28,
    Select = 0x29,
    Print = 0x2A,
    Execute = 0x2B,
    Snapshot = 0x2C,
    Insert = 0x2D,
    Delete = 0x2E,
    Help = 0x2F,
    //
    _0 = 0x30,
    _1 = 0x31,
    _2 = 0x32,
    _3 = 0x33,
    _4 = 0x34,
    _5 = 0x35,
    _6 = 0x36,
    _7 = 0x37,
    _8 = 0x38,
    _9 = 0x39,
    //
    A = 0x41,
    B = 0x42,
    C = 0x43,
    D = 0x44,
    E = 0x45,
    F = 0x46,
    G = 0x47,
    H = 0x48,
    I = 0x49,
    J = 0x4A,
    K = 0x4B,
    L = 0x4C,
    M = 0x4D,
    N = 0x4E,
    O = 0x4F,
    P = 0x50,
    Q = 0x51,
    R = 0x52,
    S = 0x53,
    T = 0x54,
    U = 0x55,
    V = 0x56,
    W = 0x57,
    X = 0x58,
    Y = 0x59,
    Z = 0x5A,
    //
    LWin = 0x5B,
    RWin = 0x5C,
    Apps = 0x5D,
    //
    Sleep = 0x5F,
    //
    NumPad0 = 0x60,
    NumPad1 = 0x61,
    NumPad2 = 0x62,
    NumPad3 = 0x63,
    NumPad4 = 0x64,
    NumPad5 = 0x65,
    NumPad6 = 0x66,
    NumPad7 = 0x67,
    NumPad8 = 0x68,
    NumPad9 = 0x69,
    Multiply = 0x6A,
    Add = 0x6B,
    Separator = 0x6C,
    Subtract = 0x6D,
    Decimal = 0x6E,
    Divide = 0x6F,
    //
    F1 = 0x70,
    F2 = 0x71,
    F3 = 0x72,
    F4 = 0x73,
    F5 = 0x74,
    F6 = 0x75,
    F7 = 0x76,
    F8 = 0x77,
    F9 = 0x78,
    F10 = 0x79,
    F11 = 0x7A,
    F12 = 0x7B,
    F13 = 0x7C,
    F14 = 0x7D,
    F15 = 0x7E,
    F16 = 0x7F,
    F17 = 0x80,
    F18 = 0x81,
    F19 = 0x82,
    F20 = 0x83,
    F21 = 0x84,
    F22 = 0x85,
    F23 = 0x86,
    F24 = 0x87,
    //
    NumLock = 0x90,
    Scroll = 0x91,
    NumPadEqual = 0x92,
    //
    LShift = 0xA0,
    RShift = 0xA1,
    LCtrl = 0xA2,
    RCtrl = 0xA3,
    LMenu = 0xA4,
    RMenu = 0xA5,
    //
    SemiColon = 0xBA,
    Plus = 0xBB,
    Comma = 0xBC,
    Minus = 0xBD,
    Period = 0xBE,
    Slash = 0xBF,
    Grave = 0xC0,
}

#[derive(Debug, Copy, Clone)]
pub enum Button {
    Left,
    Right,
    Middle,
    MaxButtons,
}

#[derive(Debug)]
pub enum Event {
    Key {
        key: Key,
    },
    Button {
        button: Button,
    },
    MousePos {
        x: i32,
        y: i32,
    },
    ConfigureNotify {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    CloseWindow,
}
