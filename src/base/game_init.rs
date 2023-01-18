//! Wrapper around [`game_init`].

use std::slice::from_raw_parts;

use super::{
    cstr_to_rust,
    sys::{
        game_init, GAME_INIT_SOURCE_TYPE_E_GAME_INIT_SOURCE_TYPE_DEFAULT as SOURCE_TYPE_DEFAULT,
        GAME_INIT_SOURCE_TYPE_E_GAME_INIT_SOURCE_TYPE_SERIALIZED as SOURCE_TYPE_SERIALIZED,
        GAME_INIT_SOURCE_TYPE_E_GAME_INIT_SOURCE_TYPE_STANDARD as SOURCE_TYPE_STANDARD,
    },
};

/// Rust version of [`game_init`] borrowing the referenced data structures.
#[derive(Debug, Copy, Clone)]
pub enum GameInit<'l> {
    Default,
    Standard {
        opts: Option<&'l str>,
        legacy: Option<&'l str>,
        state: Option<&'l str>,
    },
    Serialized(&'l [u8]),
}

impl<'l> GameInit<'l> {
    /// Create a new [`GameInit`] from a [`game_init`].
    ///
    /// # Safety
    /// The supplied `init_info` must be valid.
    pub unsafe fn new(init_info: &game_init) -> Self {
        match init_info.source_type {
            SOURCE_TYPE_DEFAULT => Self::Default,
            SOURCE_TYPE_STANDARD => {
                let source = init_info.source.standard;
                Self::Standard {
                    opts: cstr_to_rust(source.opts),
                    legacy: cstr_to_rust(source.legacy),
                    state: cstr_to_rust(source.state),
                }
            }
            SOURCE_TYPE_SERIALIZED => {
                let source = init_info.source.serialized;
                let begin: *const u8 = source.b.data.cast::<u8>();
                Self::Serialized(from_raw_parts(begin, source.b.len))
            }
            _ => unreachable!("unexpected SOURCE_TYPE"),
        }
    }
}
