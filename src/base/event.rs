//! Wrapper for the _mirabel_ event framework.

use crate::MoveDataSync;

pub use super::{sys::game_methods, sys::move_code, sys::player_id};

use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::null_mut,
    slice::from_raw_parts,
};

use super::{
    game_init::GameInit, sys::move_data_s__bindgen_ty_1 as move_data_cl, sys::*, ValidCStr,
};

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

    /// Create a new game move event by coping from the `player` and the `mov`.
    pub fn new_game_move(player: player_id, mov: MoveDataSync<MoveData>) -> Self {
        let mut event = MaybeUninit::<event_any>::uninit();
        unsafe {
            event_create_game_move(event.as_mut_ptr(), player, mov.into());
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
    GameMove(EventGameMove<'l>),
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

pub struct EventGameMove<'l> {
    pub base: Event,
    pub player: player_id,
    pub data: MoveDataSync<MoveData<'l>>,
}

impl<'l> EventGameMove<'l> {
    unsafe fn new(event: &'l event_game_move) -> Self {
        Self {
            base: Event::new(&event.base),
            player: event.player,
            data: MoveDataSync {
                md: MoveData::from_ref(&event.data.md),
                sync_ctr: event.data.sync_ctr,
            },
        }
    }
}

/// Rust equivalent of a borrowed [`move_data`].
#[derive(Clone, Copy)]
pub enum MoveData<'l> {
    MoveCode(move_code),
    BigMove(&'l [u8]),
}

impl<'l> MoveData<'l> {
    /// Converts a valid [`move_data`] to a [`Self`] by shallow-copying.
    #[inline]
    unsafe fn from_ref(md: &move_data) -> Self {
        // A move is a big move iff data!=NULL.
        if md.data.is_null() {
            Self::MoveCode(md.cl.code)
        } else if md.cl.len == 0 {
            // Handle this case separately, just in case.
            Self::BigMove(&[])
        } else {
            Self::BigMove(from_raw_parts(md.data, md.cl.len))
        }
    }
}

impl<'l> From<MoveData<'l>> for move_data {
    #[inline]
    fn from(value: MoveData<'l>) -> Self {
        match value {
            MoveData::MoveCode(code) => move_data {
                cl: move_data_cl { code },
                data: null_mut(),
            },
            MoveData::BigMove(slice) => move_data {
                cl: move_data_cl { len: slice.len() },
                // Slice pointers are never NULL as required for big moves.
                // data is only read afterwards. Hence, it is fine to cast here.
                data: slice.as_ptr().cast_mut(),
            },
        }
    }
}
