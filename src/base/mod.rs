//! Generated bindings for the _mirabel_ and _surena_ plugin APIs.
//!
//! This module also provides some helpers.

pub mod error;
pub mod game_init;
pub mod string;
pub mod sys;

#[cfg(feature = "mirabel")]
pub mod event;

#[cfg(feature = "mirabel")]
pub mod log;

pub use string::*;

/// Generic struct for creating [`move_data_sync`](sys::move_data_sync) wrappers.
///
/// This will match the layout of [`move_data_sync`](sys::move_data_sync) if M
/// matches [`move_data`](sys::move_data).
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MoveDataSync<M> {
    pub md: M,
    pub sync_ctr: u64,
}

impl<M> MoveDataSync<M> {
    /// Create a new [`Self`] using the
    /// [`SYNC_CTR_DEFAULT`](sys::SYNC_CTR_DEFAULT).
    ///
    /// This is mainly a helper for perfect-information games.
    ///
    /// # Example
    /// ```
    /// # use mirabel::{MoveDataSync, sys::SYNC_CTR_DEFAULT};
    /// assert_eq!(
    ///     MoveDataSync {
    ///         md: 42,
    ///         sync_ctr: SYNC_CTR_DEFAULT
    ///     },
    ///     MoveDataSync::with_default(42)
    /// );
    /// ```
    pub fn with_default(md: M) -> Self {
        Self {
            md,
            sync_ctr: sys::SYNC_CTR_DEFAULT,
        }
    }
}

impl<M: Into<sys::move_data>> From<MoveDataSync<M>> for sys::move_data_sync {
    #[inline]
    fn from(value: MoveDataSync<M>) -> Self {
        sys::move_data_sync {
            md: value.md.into(),
            sync_ctr: value.sync_ctr,
        }
    }
}

/// Simple macro for counting the number of provided arguments.
///
/// # Example
/// ```
/// # use mirabel::count;
/// assert_eq!(3, count!(1, "AB", true));
/// ```
#[macro_export]
macro_rules! count {
    () => { 0 };
    ($_e: tt $(, $rest: tt)*) => { 1 + $crate::count!($($rest),*) }
}
