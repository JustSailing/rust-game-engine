use std::os::raw::c_void;

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
}

pub enum EventCodes {
    ApplicationQuit = 0x01,
    KeyPressed = 0x02,
    KeyReleased = 0x03,
    ButtonPressed = 0x04,
    ButtonReleased = 0x05,
    MouseMoved = 0x06,
    MouseWheel = 0x07,
    WindowResized = 0x08,
    //Dont touch
    MaxCodes,
}

type PfnOnEvent =
    fn(code: usize, sender: *const c_void, listener: *const c_void, data: &EventCtx) -> bool;

#[derive(Clone, Copy, Debug)]
struct RegisteredEvent {
    listener: *const c_void,
    callback: PfnOnEvent,
}

#[derive(Clone, Debug)]
struct EventCodeEntry {
    events: Vec<RegisteredEvent>,
}

pub enum EventError {
    AlreadyInitialized,
    NotInitialized,
    AlreadyShutdown,
}

static mut EVENT_STATE: Option<EventState> = None;

pub struct EventState {
    registered: [EventCodeEntry; EventCodes::MaxCodes as usize],
}

impl EventState {
    pub fn initialize() -> Result<(), EventError> {
        unsafe {
            if let Some(ref _a) = EVENT_STATE {
                return Err(EventError::AlreadyInitialized);
            }
        };

        let codes: [EventCodeEntry; EventCodes::MaxCodes as usize] =
            std::iter::repeat_with(|| EventCodeEntry { events: Vec::new() })
                .take(EventCodes::MaxCodes as usize)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

        let state: EventState = EventState { registered: codes };

        unsafe { EVENT_STATE = Some(state) };

        Ok(())
    }

    pub fn shutdown() -> Result<(), EventError> {
        unsafe {
            if let Some(ref mut state) = EVENT_STATE {
                for elem in &mut state.registered {
                    elem.events.clear();
                }
                EVENT_STATE = None;
            } else {
                return Err(EventError::AlreadyShutdown);
            }
        };

        Ok(())
    }

    pub fn register_event(
        code: usize,
        listener: *const c_void,
        on_event: PfnOnEvent,
    ) -> Result<(), EventError> {
        let state = unsafe {
            if let Some(ref mut s) = EVENT_STATE {
                s
            } else {
                return Err(EventError::NotInitialized);
            }
        };

        for v in &state.registered[code].events {
            if v.listener == listener {
                // TODO: Warn
                return Ok(());
            }
        }

        state.registered[code].events.push(RegisteredEvent {
            listener: listener,
            callback: on_event,
        });
        Ok(())
    }

    pub fn unregister_event(
        code: usize,
        listener: *const c_void,
        on_event: PfnOnEvent,
    ) -> Result<(), EventError> {
        let state = unsafe {
            if let Some(ref mut s) = EVENT_STATE {
                s
            } else {
                return Err(EventError::NotInitialized);
            }
        };

        let mut i: usize = 0;
        let mut check: bool = false;

        for (it, reg) in state.registered[code].events.iter().enumerate() {
            if reg.listener == listener && std::ptr::fn_addr_eq(reg.callback, on_event) {
                check = true;
                i = it;
                break;
            }
        }

        if check {
            state.registered[code].events.remove(i);
        }

        Ok(())
    }

    pub fn fire_event(
        code: usize,
        sender: *const c_void,
        ctx: &EventCtx,
    ) -> Result<(), EventError> {
        let state = unsafe {
            if let Some(ref mut s) = EVENT_STATE {
                s
            } else {
                return Err(EventError::NotInitialized);
            }
        };
        for reg in &state.registered[code].events {
            if (reg.callback)(code, sender, reg.listener, &ctx) {
                return Ok(());
            }
        }
        Ok(())
    }
}
