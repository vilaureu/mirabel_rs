//! This package presents wrapper for writing _mirabel_ plugins in (mostly) safe
//! Rust.
//!
//! # Features
//! - `frontend`: Provide a wrapper for writing frontend plugins.
//! - `skia`: Provide a _Skia_ wrapper for drawing in the frontend.

#[cfg(feature = "frontend")]
pub mod frontend;

#[cfg(any(feature = "frontend"))]
pub mod sdl_event;

#[cfg(feature = "skia")]
mod skia_helper;

pub use mirabel_sys::{
    self, count, cstr,
    error::*,
    event::*,
    imgui,
    log::mirabel_log,
    sys::{self, semver},
    ValidCStr,
};

#[cfg(any(feature = "frontend"))]
pub use sdl_event::SDLEventEnum;

/// On error, this stores an [`ErrorCode`] and no message.
pub type CodeResult<T> = std::result::Result<T, ErrorCode>;
