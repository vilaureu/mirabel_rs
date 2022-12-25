//! This module presents wrappers for writing _mirabel_ plugins in (mostly) safe
//! Rust.

pub mod frontend;
pub mod imgui;
pub mod sdl_event;

#[cfg(feature = "skia")]
mod skia_helper;

use crate::error::ErrorCode;

/// On error, this stores an [`ErrorCode`] and no message.
pub type CodeResult<T> = std::result::Result<T, ErrorCode>;
