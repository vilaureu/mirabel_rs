//! This crate provides a Rust wrapper for writing
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) and
//! [_surena_](https://github.com/RememberOfLife/surena/) plugins.
//!
//! # Features
//! - `mirabel`: Include support for _mirabel_ (GUI) plugins. Else, only
//!   _surena_ wrappers are available.
//! - `skia`: Provide a _Skia_ wrapper for drawing in the frontend.

mod base;
mod surena;

#[cfg(feature = "mirabel")]
mod gui;

pub use base::*;
pub use surena::*;

#[cfg(feature = "mirabel")]
pub use gui::*;
