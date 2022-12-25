//! Wrapper for the _mirabel_ event framework.

pub use super::{game_init::GameInit, sys::game_methods, sys::move_code, sys::player_id};

use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use super::{sys::*, ValidCStr};

/// Wrapper for an owned [`event_any`].
///
/// This guarantees that the wrapped event is valid and will destroy the event
/// on drop.
pub struct EventAny(event_any);

impl EventAny {
    /// Create a new [`EventAny`] from an [`event_any`].
    ///
    /// # Safety
    /// The supplied `event` must be valid.
    #[inline]
    pub unsafe fn new(event: event_any) -> Self {
        Self(event)
    }

    #[inline]
    pub fn get_type(&self) -> EVENT_TYPE {
        unsafe { self.base.type_ }
    }

    pub fn to_rust(&self) -> EventEnum {
        unsafe { EventEnum::new(self) }
    }

    pub fn new_game_move(player: player_id, code: move_code) -> Self {
        let mut event = MaybeUninit::<event_any>::uninit();
        unsafe {
            // This sets the sync_counter to 0 because it is ignored in events
            // originating from plugins anyway.
            event_create_game_move(event.as_mut_ptr(), 0, player, code);
        }
        unsafe { Self(event.assume_init()) }
    }
}

impl Deref for EventAny {
    type Target = event_any;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EventAny {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for EventAny {
    fn drop(&mut self) {
        unsafe { event_destroy(&mut **self) };
    }
}

/// _mirabel_ event converted to a Rust enum.
#[non_exhaustive]
pub enum EventEnum<'l> {
    GameLoadMethods(EventGameLoadMethods<'l>),
    GameUnload(Event),
    GameState(EventGameState<'l>),
    GameMove(EventGameMove),
    Unknown,
}

impl<'l> EventEnum<'l> {
    /// Create a new [`EventEnum`] from an [`event_any`].
    ///
    /// # Safety
    /// The supplied `event` must be valid.
    unsafe fn new(event: &'l event_any) -> Self {
        match event.base.type_ {
            EVENT_TYPE_E_EVENT_TYPE_GAME_LOAD_METHODS => {
                Self::GameLoadMethods(EventGameLoadMethods::new(&event.game_load_methods))
            }
            EVENT_TYPE_E_EVENT_TYPE_GAME_UNLOAD => Self::GameUnload(Event::new(&event.base)),
            EVENT_TYPE_E_EVENT_TYPE_GAME_STATE => {
                Self::GameState(EventGameState::new(&event.game_state))
            }
            EVENT_TYPE_E_EVENT_TYPE_GAME_MOVE => {
                Self::GameMove(EventGameMove::new(&event.game_move))
            }
            _ => Self::Unknown,
        }
    }
}
pub struct Event {
    pub type_: EVENT_TYPE,
    pub client_id: u32,
    pub lobby_id: u32,
}

impl Event {
    unsafe fn new(event: &event) -> Self {
        Self {
            type_: event.type_,
            client_id: event.client_id,
            lobby_id: event.lobby_id,
        }
    }
}
pub struct EventGameLoadMethods<'l> {
    pub base: Event,
    // TODO: Provide safe wrapper for game_methods.
    pub methods: *const game_methods,
    pub init_info: GameInit<'l>,
}

impl<'l> EventGameLoadMethods<'l> {
    unsafe fn new(event: &'l event_game_load_methods) -> Self {
        Self {
            base: Event::new(&event.base),
            methods: event.methods,
            init_info: GameInit::new(&event.init_info),
        }
    }
}

pub struct EventGameState<'l> {
    pub base: Event,
    pub state: Option<ValidCStr<'l>>,
}

impl<'l> EventGameState<'l> {
    unsafe fn new(event: &'l event_game_state) -> Self {
        Self {
            base: Event::new(&event.base),
            state: ValidCStr::new(event.state),
        }
    }
}

pub struct EventGameMove {
    pub base: Event,
    pub player: player_id,
    pub code: move_code,
}

impl EventGameMove {
    unsafe fn new(event: &event_game_move) -> Self {
        Self {
            base: Event::new(&event.base),
            player: event.player,
            code: event.code,
        }
    }
}
