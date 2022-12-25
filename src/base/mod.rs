//! Generated bindings for the _mirabel_ and _surena_ plugin APIs.
//!
//! This module also provides some helpers.

pub mod error;
pub mod game_init;
pub mod ptr_vec;
pub mod string;
pub mod sys;

#[cfg(feature = "mirabel")]
pub mod event;

#[cfg(feature = "mirabel")]
pub mod log;

pub use string::*;

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
    ($_e: expr $(, $rest: expr)*) => { 1 + $crate::count!($($rest),*) }
}
