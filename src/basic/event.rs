use std::{os::raw::c_void, sync::mpsc::Sender};
use thiserror::Error;

type Result<T> = std::result::Result<T, EventSysError>;

#[derive(Clone, Copy)]
pub enum EventCtx {
    I64([i64; 2]),
    U64([u64; 2]),
    F64([f64; 2]),

    I32([i32; 4]),
    U32([u32; 4]),
    F32([f32; 4]),

    I16([i16; 8]),
    U16([u16; 8]),

    I8([i8; 16]),
    U8([u8; 16]),

    Char([char; 16]),

    None,
}

#[derive(Debug)]
pub enum EventCodes {
    ApplicationQuit = 0x01,
    KeyPressed = 0x02,
    KeyReleased = 0x03,
    ButtonPressed = 0x04,
    ButtonReleased = 0x05,
    MouseMoved = 0x06,
    MouseWheel = 0x07,
    WindowResized = 0x08,
    SetRenderMode = 0x09,
    Debug0 = 0x15,
    //Dont touch
    MaxCodes,
}

impl From<usize> for EventCodes {
    fn from(value: usize) -> Self {
        match value {
            0x01 => EventCodes::ApplicationQuit,
            0x02 => EventCodes::KeyPressed,
            0x03 => EventCodes::KeyReleased,
            0x04 => EventCodes::ButtonPressed,
            0x05 => EventCodes::ButtonReleased,
            0x06 => EventCodes::MouseMoved,
            0x07 => EventCodes::MouseWheel,
            0x08 => EventCodes::WindowResized,
            0x09 => EventCodes::SetRenderMode,
            0x15 => EventCodes::Debug0,
            _ => EventCodes::MaxCodes,
        }
    }
}

#[derive(Error, Debug)]
pub enum EventSysError {
    #[error("event system error: already initialize {} {}", file!(), line!())]
    AlreadyInitialized,
    #[error("event system error: not intialized {} {}", file!(), line!())]
    NotInitialized,
    #[error("event system error: already shutdown {} {}", file!(), line!())]
    AlreadyShutdown,
    #[error("event system error: channel send failed")]
    SendError,
}

pub type EventTuple = (usize, EventCtx);

pub struct EventSystem {
    sender: Sender<EventTuple>,
}

impl EventSystem {
    pub fn initialize(sender: Sender<EventTuple>) -> Result<Self> {
        Ok(Self { sender: sender })
    }

    pub fn fire_event(
        &mut self,
        code: usize,
        _sender: *const c_void,
        ctx: &EventCtx,
    ) -> Result<()> {
        match self.sender.send((code, ctx.clone())) {
            Ok(_) => Ok(()),
            Err(_) => Err(EventSysError::SendError),
        }
    }
}
