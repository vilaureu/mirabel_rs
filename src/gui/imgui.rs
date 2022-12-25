//! Wrapper around the `imgui_c_thin` _ImGui_ wrapper of _mirabel_.
//!
//! This can be used to create _ImGui_ UIs in _mirabel_.

use std::{
    ffi::{c_char, c_void},
    mem,
};

use crate::{sys, ValidCStr};

/// Create a line of text in the UI.
pub fn text(text: &str) {
    let text = text.as_bytes().as_ptr_range();
    let start: *const u8 = text.start;
    let end: *const u8 = text.end;
    unsafe {
        sys::ImGuiThin_TextUnformatted(start.cast::<c_char>(), end.cast::<c_char>());
    }
}

/// Create a button with `label`.
///
/// This returns true once if pressed.
pub fn button(label: ValidCStr) -> bool {
    unsafe { sys::ImGuiThin_Button(label.into()) }
}

/// Create a check box with `label`.
///
/// This returns the current state of the check_box.
pub fn check_box(label: ValidCStr, state: &mut bool) -> bool {
    unsafe { sys::ImGuiThin_CheckBox(label.into(), state) }
}

/// Create an input box with `label` of content length `size`.
///
/// This displays `string` and also stores the new contents in `string` if
/// changed.
/// Returns `true` if changed.
///
/// # Example
/// ```no_run
/// # use mirabel::{cstr, imgui::input_text};
/// let mut string = "Enter Characters Here".to_string();
/// let changed = input_text(cstr("my input\0"), &mut string, 256);
/// assert!(changed);
/// assert_eq!("Some Characters", &string);
/// ```
pub fn input_text(label: ValidCStr, string: &mut String, size: usize) -> bool {
    let mut buf = mem::take(string).into_bytes();
    let orig_len = buf.len();
    buf.resize(buf.len().max(size), 0);
    buf.push(0);

    let buf_size = buf.len();
    let ptr: *mut u8 = buf.as_mut_slice().as_mut_ptr();
    let changed = unsafe { sys::ImGuiThin_InputText(label.into(), ptr.cast::<c_char>(), buf_size) };

    let new_len = if changed {
        buf.iter()
            .enumerate()
            .find(|(_, b)| **b == 0)
            .expect("ImGui InputText buffer not NUL-terminated")
            .0
    } else {
        orig_len
    };
    buf.truncate(new_len);
    *string = unsafe { String::from_utf8_unchecked(buf) };

    changed
}

/// Create a slider with `label` going from `min` to `max`.
///
/// Displays the `value` and also stores the updated value in `value` if
/// changed.
/// Returns `true` if changed.
///
/// # Example
/// ```no_run
/// # use mirabel::{cstr, imgui::slider_scalar};
/// let mut value = 42;
/// let changed = slider_scalar::<u8>(cstr("my slider\0"), &mut value, 4, 255);
/// assert!(changed);
/// assert_eq!(76, value);
/// ```
pub fn slider_scalar<D: DataType>(label: ValidCStr, value: &mut D, mut min: D, mut max: D) -> bool {
    let value: *mut D = value;
    // FIXME: Make pointers constant.
    let min: *mut D = &mut min;
    let max: *mut D = &mut max;
    unsafe {
        sys::ImGuiThin_SliderScalar(
            label.into(),
            D::enum_value(),
            value.cast::<c_void>(),
            min.cast::<c_void>(),
            max.cast::<c_void>(),
        )
    }
}

/// Create an input for a scalar `D`.
///
/// Displays `value`.
///
/// Returns `true` if changed and stores the updated value in `value`.
pub fn input_scalar<D: DataType>(label: ValidCStr, value: &mut D) -> bool {
    let value: *mut D = value;
    unsafe { sys::ImGuiThin_InputScalar(label.into(), D::enum_value(), value.cast::<c_void>()) }
}

/// Disable all UI elements between [`begin_disabled`] and [`end_disabled`].
///
/// Only disables the elements if `disable` is `true`.
/// Must always be matched with an [`end_disabled`].
pub fn begin_disabled(disable: bool) {
    unsafe {
        sys::ImGuiThin_BeginDisabled(disable);
    }
}

/// Cancel a [`begin_disabled`].
///
/// # Panics
/// Aborts if called without matching [`begin_disabled()`].
pub fn end_disabled() {
    unsafe {
        sys::ImGuiThin_EndDisabled();
    }
}

/// Scalars which are accepted by _ImGui_.
///
/// # Safety
/// Must only be implemented on _ImGui_ scalars.
/// [`Self::enum_value()`] must return the correct [`IMGUITHIN_DATATYPE`](sys::IMGUITHIN_DATATYPE).
pub unsafe trait DataType {
    fn enum_value() -> u32;
}

macro_rules! data_type {
    ($im:expr => $rust:ty) => {
        unsafe impl DataType for $rust {
            fn enum_value() -> u32 {
                $im
            }
        }
    };
}

data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_S8 => i8);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_U8 => u8);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_S16 => i16);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_U16 => u16);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_S32 => i32);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_U32 => u32);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_S64 => i64);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_U64 => u64);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_FLOAT => f32);
data_type!(sys::IMGUITHIN_DATATYPE_E_IMGUITHIN_DATATYPE_DOUBLE => f64);
