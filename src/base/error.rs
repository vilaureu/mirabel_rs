//! Helpers for error handling in plugin APIs.

use std::{ffi::c_char, fmt::Display, num::NonZeroU32, ptr::null};

use super::{
    cstr, cstr_to_rust,
    sys::{self, error_code, ERR_ERR_ENUM_DEFAULT_OFFSET},
    ValidCStr, ValidCString,
};

/// Type for C-compatible error strings.
///
/// This allows to have no error string ([`ErrorString::None`]), a static error
/// string ([`ErrorString::Static`]), or a dynamic error string
/// ([`ErrorString::Dynamic`]).
#[derive(Debug)]
pub enum ErrorString {
    None,
    Static(ValidCStr<'static>),
    Dynamic(ValidCString),
}

impl Default for ErrorString {
    #[inline]
    fn default() -> Self {
        ErrorString::None
    }
}

impl From<&ErrorString> for *const c_char {
    fn from(e: &ErrorString) -> Self {
        match e {
            ErrorString::None => null(),
            ErrorString::Static(s) => (*s).into(),
            ErrorString::Dynamic(s) => s.as_ptr(),
        }
    }
}

/// Error type for API functions.
///
/// The APIs always expect an error code and optionally an error message.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub message: ErrorString,
}

impl Error {
    /// Create an error from a static C string.
    ///
    /// # Panics
    /// Panics if the byte string is not NUL-terminated.
    ///
    /// # Example
    /// ```
    /// # use mirabel::error::*;
    /// Error::new_static(ErrorCode::InvalidInput, "state string malformed\0");
    /// ```
    #[inline]
    pub fn new_static(code: ErrorCode, message: &'static str) -> Self {
        Error {
            code,
            message: ErrorString::Static(cstr(message)),
        }
    }

    /// Create an error from a [`String`].
    ///
    /// # Panics
    /// This function will panic if `message` contains a NUL byte.
    ///
    /// # Example
    /// ```
    /// # use mirabel::error::*;
    /// Error::new_dynamic(ErrorCode::InvalidOptions, format!("board size larger than {}", 42));
    /// ```
    pub fn new_dynamic(code: ErrorCode, message: String) -> Self {
        Error {
            code,
            message: ErrorString::Dynamic(message.try_into().expect("msg")),
        }
    }
}

impl From<ErrorCode> for Error {
    /// Create an error without a `message`.
    #[inline]
    fn from(code: ErrorCode) -> Self {
        Self {
            code,
            message: Default::default(),
        }
    }
}

/// _surena_ error codes as a Rust enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorCode {
    StateUnrecoverable,
    StateCorrupted,
    OutOfMemory,
    FeatureUnsupported,
    MissingHiddenState,
    InvalidInput,
    InvalidPlayer,
    InvalidMove,
    InvalidOptions,
    InvalidLegacy,
    InvalidState,
    UnstablePosition,
    Retry,
    CustomAny,
    Custom(CustomCode),
}

impl From<NonZeroU32> for ErrorCode {
    #[inline]
    fn from(code: NonZeroU32) -> Self {
        assert_eq!(0, sys::ERR_ERR_OK);
        match code.get() {
            0 => unreachable!(),
            sys::ERR_ERR_STATE_UNRECOVERABLE => ErrorCode::StateUnrecoverable,
            sys::ERR_ERR_STATE_CORRUPTED => ErrorCode::StateCorrupted,
            sys::ERR_ERR_OUT_OF_MEMORY => ErrorCode::OutOfMemory,
            sys::ERR_ERR_FEATURE_UNSUPPORTED => ErrorCode::FeatureUnsupported,
            sys::ERR_ERR_MISSING_HIDDEN_STATE => ErrorCode::MissingHiddenState,
            sys::ERR_ERR_INVALID_INPUT => ErrorCode::InvalidInput,
            sys::ERR_ERR_INVALID_PLAYER => ErrorCode::InvalidPlayer,
            sys::ERR_ERR_INVALID_MOVE => ErrorCode::InvalidMove,
            sys::ERR_ERR_INVALID_OPTIONS => ErrorCode::InvalidOptions,
            sys::ERR_ERR_INVALID_LEGACY => ErrorCode::InvalidLegacy,
            sys::ERR_ERR_INVALID_STATE => ErrorCode::InvalidState,
            sys::ERR_ERR_UNSTABLE_POSITION => ErrorCode::UnstablePosition,
            sys::ERR_ERR_RETRY => ErrorCode::Retry,
            sys::ERR_ERR_CUSTOM_ANY => ErrorCode::CustomAny,
            ERR_ERR_ENUM_DEFAULT_OFFSET.. => {
                ErrorCode::Custom(CustomCode::new(code.get()).unwrap())
            }
        }
    }
}

impl From<ErrorCode> for error_code {
    #[inline]
    fn from(error: ErrorCode) -> Self {
        match error {
            ErrorCode::StateUnrecoverable => sys::ERR_ERR_STATE_UNRECOVERABLE,
            ErrorCode::StateCorrupted => sys::ERR_ERR_STATE_CORRUPTED,
            ErrorCode::OutOfMemory => sys::ERR_ERR_OUT_OF_MEMORY,
            ErrorCode::FeatureUnsupported => sys::ERR_ERR_FEATURE_UNSUPPORTED,
            ErrorCode::MissingHiddenState => sys::ERR_ERR_MISSING_HIDDEN_STATE,
            ErrorCode::InvalidInput => sys::ERR_ERR_INVALID_INPUT,
            ErrorCode::InvalidPlayer => sys::ERR_ERR_INVALID_PLAYER,
            ErrorCode::InvalidMove => sys::ERR_ERR_INVALID_MOVE,
            ErrorCode::InvalidOptions => sys::ERR_ERR_INVALID_OPTIONS,
            ErrorCode::InvalidLegacy => sys::ERR_ERR_INVALID_LEGACY,
            ErrorCode::InvalidState => sys::ERR_ERR_INVALID_STATE,
            ErrorCode::UnstablePosition => sys::ERR_ERR_UNSTABLE_POSITION,
            ErrorCode::Retry => sys::ERR_ERR_RETRY,
            ErrorCode::CustomAny => sys::ERR_ERR_CUSTOM_ANY,
            ErrorCode::Custom(code) => code.into(),
        }
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = error_code::from(*self);
        let error = unsafe { cstr_to_rust(sys::get_general_error_string(code)) };
        match error {
            Some(s) => write!(f, "{}", s),
            None => write!(f, "custom[{}]", code),
        }
    }
}

/// Wrapper for custom error codes.
///
/// It assures that the error code is not smaller than [`ERR_ERR_ENUM_DEFAULT_OFFSET`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CustomCode(NonZeroU32);

impl CustomCode {
    /// Create a new custom error code.
    ///
    /// Returns [`None`] if `code` is smaller than [`ERR_ERR_ENUM_DEFAULT_OFFSET`].
    #[inline]
    pub fn new(code: error_code) -> Option<Self> {
        if code >= ERR_ERR_ENUM_DEFAULT_OFFSET {
            let code = code.try_into().ok()?;
            Some(Self(code))
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self) -> error_code {
        self.0.get()
    }
}

impl From<CustomCode> for error_code {
    #[inline]
    fn from(code: CustomCode) -> Self {
        code.get()
    }
}

/// Result type using the special [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Transform an [`error_code`] to a [`Result`](std::result::Result) with
/// [`ErrorCode`].
///
/// # Example
/// ```
/// # use mirabel::{sys::{ERR_ERR_MISSING_HIDDEN_STATE, ERR_ERR_OK}, error::*};
/// assert_eq!(
///     Ok(()),
///     code_to_result(ERR_ERR_OK),
/// );
/// assert_eq!(
///     Err(ErrorCode::MissingHiddenState),
///     code_to_result(ERR_ERR_MISSING_HIDDEN_STATE),
/// );
/// ```
#[inline]
pub fn code_to_result(code: error_code) -> std::result::Result<(), ErrorCode> {
    match NonZeroU32::new(code) {
        Some(c) => Err(c.into()),
        None => Ok(()),
    }
}
