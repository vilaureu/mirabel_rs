//! Wrapper around _SDL_ events.

use std::fmt;

use mirabel_sys::sys::{self, SDL_Event, SDL_WindowEvent};

pub use mirabel_sys::sys::{
    SDL_KeyboardEvent, SDL_MouseButtonEvent, SDL_MouseMotionEvent, SDL_MouseWheelEvent,
};

/// An _SDL_ event.
#[non_exhaustive]
pub enum SDLEventEnum {
    WindowEvent(SDL_WindowEvent),
    KeyDown(SDL_KeyboardEvent),
    KeyUp(SDL_KeyboardEvent),
    MouseMotion(SDL_MouseMotionEvent),
    MouseButtonDown(SDL_MouseButtonEvent),
    MouseButtonUp(SDL_MouseButtonEvent),
    MouseWheel(SDL_MouseWheelEvent),
    /// All other events.
    Unknown(SDL_Event),
}

impl SDLEventEnum {
    #[inline]
    pub(crate) unsafe fn new(event: SDL_Event) -> Self {
        match event.type_ {
            sys::SDL_EventType_SDL_WINDOWEVENT => Self::WindowEvent(event.window),
            sys::SDL_EventType_SDL_KEYDOWN => Self::KeyDown(event.key),
            sys::SDL_EventType_SDL_KEYUP => Self::KeyUp(event.key),
            sys::SDL_EventType_SDL_MOUSEMOTION => Self::MouseMotion(event.motion),
            sys::SDL_EventType_SDL_MOUSEBUTTONDOWN => Self::MouseButtonDown(event.button),
            sys::SDL_EventType_SDL_MOUSEBUTTONUP => Self::MouseButtonUp(event.button),
            sys::SDL_EventType_SDL_MOUSEWHEEL => Self::MouseWheel(event.wheel),
            _ => Self::Unknown(event),
        }
    }
}

impl fmt::Debug for SDLEventEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Unknown {
            type_: u32,
        }
        impl fmt::Debug for Unknown {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("SDL_event")
                    .field("type_", &self.type_)
                    .finish_non_exhaustive()
            }
        }

        match self {
            Self::WindowEvent(e) => f.debug_tuple("WindowEvent").field(e).finish(),
            Self::KeyDown(e) => f.debug_tuple("KeyDown").field(e).finish(),
            Self::KeyUp(e) => f.debug_tuple("KeyUp").field(e).finish(),
            Self::MouseMotion(e) => f.debug_tuple("MouseMotion").field(e).finish(),
            Self::MouseButtonDown(e) => f.debug_tuple("MouseButtonDown").field(e).finish(),
            Self::MouseButtonUp(e) => f.debug_tuple("MouseButtonUp").field(e).finish(),
            Self::MouseWheel(e) => f.debug_tuple("MouseWheel").field(e).finish(),
            Self::Unknown(e) => f
                .debug_tuple("Unknown")
                .field(&Unknown {
                    type_: unsafe { e.type_ },
                })
                .finish(),
        }
    }
}
