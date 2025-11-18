use std::{cell::RefCell, os::raw::c_void, rc::Rc};
use thiserror::Error;

type Result<T> = std::result::Result<T, EventSysError>;

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

pub trait EventCallback {
    fn handle_event(
        &mut self,
        code: usize,
        sender: *const c_void,
        listener: *const c_void,
        data: &EventCtx,
    ) -> bool;
}

type PfnOnEvent =
    fn(code: usize, sender: *const c_void, listener: *const c_void, data: &EventCtx) -> bool;

//#[derive(Default)]
struct RegisteredEvent<'a> {
    listener: *const c_void,
    callback: Box<Rc<RefCell<dyn EventCallback + 'a>>>,
}
#[derive(Default)]
struct EventCodeEntry<'a> {
    events: Vec<RegisteredEvent<'a>>,
}

#[derive(Error, Debug)]
pub enum EventSysError {
    #[error("event system error: already initialize {} {}", file!(), line!())]
    AlreadyInitialized,
    #[error("event system error: not intialized {} {}", file!(), line!())]
    NotInitialized,
    #[error("event system error: already shutdown {} {}", file!(), line!())]
    AlreadyShutdown,
}

pub struct EventSystem<'a> {
    registered: [EventCodeEntry<'a>; EventCodes::MaxCodes as usize],
}

impl<'a> EventSystem<'a> {
    pub fn initialize() -> Result<Self> {
        let codes: [EventCodeEntry; EventCodes::MaxCodes as usize] =
            [(); EventCodes::MaxCodes as usize].map(|_| EventCodeEntry { events: Vec::new() });

        Ok(EventSystem { registered: codes })
    }

    pub fn register_event(
        &mut self,
        code: usize,
        listener: *const c_void,
        on_event: Box<Rc<RefCell<dyn EventCallback + 'a>>>,
    ) -> Result<()> {
        for v in &self.registered[code].events {
            if v.listener == listener {
                // TODO: Warn
                return Ok(());
            }
        }

        self.registered[code].events.push(RegisteredEvent {
            listener,
            callback: on_event,
        });
        Ok(())
    }

    //FIXME
    pub fn unregister_event(
        &mut self,
        code: usize,
        listener: *const c_void,
        _on_event: Box<Rc<RefCell<dyn EventCallback + 'a>>>,
    ) -> Result<()> {
        let mut i: usize = 0;
        let mut check: bool = false;

        for (it, reg) in self.registered[code].events.iter().enumerate() {
            if reg.listener == listener
            //&& (_on_event.borrow() as Any).type_id() == (reg.callback.borrow() as Any).type_id()
            {
                check = true;
                i = it;
                break;
            }
        }

        if check {
            self.registered[code].events.remove(i);
        }

        Ok(())
    }

    pub fn fire_event(&mut self, code: usize, sender: *const c_void, ctx: &EventCtx) -> Result<()> {
        for reg in &mut self.registered[code].events {
            if reg
                .callback
                .borrow_mut()
                .handle_event(code, sender, reg.listener, &ctx)
            {
                return Ok(());
            }
        }
        Ok(())
    }
}
