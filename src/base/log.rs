//! Wrapper for the _mirabel_ log framework.

use std::ffi::c_char;

use super::sys;

/// Log via the _mirabel_ log framework.
///
/// To prevent accidental coloring, all log messages start with "+ ".
/// The wrapper will append a newline for you.
pub fn mirabel_log(msg: &str) {
    let msg = format!("+ {msg}\n");
    let msg = msg.as_bytes().as_ptr_range();
    let start: *const u8 = msg.start;
    let end: *const u8 = msg.end;
    unsafe {
        sys::mirabel_log(start.cast::<c_char>(), end.cast::<c_char>());
    }
}
