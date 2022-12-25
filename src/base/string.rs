//! Helpers for dealing with C-style strings safely.

use std::{
    ffi::{c_char, CStr, CString, FromBytesWithNulError, NulError},
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    str::from_utf8_unchecked,
};

/// Simple helper function to create a [`ValidCStr`] from an [`str`].
///
/// # Panics
/// `string` must be NUL-terminated and must not contain any other NUL byte.
///
/// # Example
/// ```
/// # use mirabel::cstr;
/// cstr("my C-style string\0");
/// ```
#[inline]
pub fn cstr(string: &str) -> ValidCStr {
    string.try_into().expect("invalid C string")
}

/// Converts raw C string pointers into Rust's [`str`]s.
///
/// # Safety
/// This requires a NULL pointer or a pointer to valid UTF-8.
/// All _surena_/_mirabel_ strings should already be valid UTF-8.
///
/// You must sure that the returned lifetime does not outlive the input data.
///
/// # Example
/// ```
/// # use mirabel::cstr_to_rust;
/// # use std::{ptr::null, ffi::c_char};
/// # unsafe {
/// assert_eq!(None, cstr_to_rust(null()));
/// let cstr = (&[b'H', b'i', b'\0']).as_ptr().cast(); // cast u8 -> c_char
/// assert_eq!(Some("Hi"), cstr_to_rust(cstr));
/// # }
/// ```
#[inline]
pub unsafe fn cstr_to_rust<'l>(cstr: *const c_char) -> Option<&'l str> {
    if cstr.is_null() {
        None
    } else {
        Some(cstr_to_rust_unchecked(cstr))
    }
}

/// Same as [`cstr_to_rust`] but does not check for NULL pointers.
///
/// # Safety
/// Same as [`cstr_to_rust`] but the pointer must not be NULL.
pub unsafe fn cstr_to_rust_unchecked<'l>(cstr: *const c_char) -> &'l str {
    if cfg!(debug_assertions) {
        // Just to catch UTF-8 validation errors in surena/mirabel.
        CStr::from_ptr(cstr).to_str().expect("invalid UTF-8")
    } else {
        from_utf8_unchecked(CStr::from_ptr(cstr).to_bytes())
    }
}

/// A C-style string with guarantees and lifetime.
///
/// This simply wraps a char pointer but guarantees that it is not NULL,
/// NUL-terminated and valid UTF-8.
#[derive(Clone, Copy)]
pub struct ValidCStr<'s> {
    cstr: NonNull<c_char>,
    phantom: PhantomData<&'s [c_char]>,
}

impl<'s> ValidCStr<'s> {
    /// Create a new ValidCStr from a char pointer or [`None`] if NULL.
    ///
    /// # Safety
    /// Make sure that the returned lifetime does not outlive the input data.
    /// The referenced character sequence must be NUL-terminated and valid
    /// UTF-8.
    #[inline]
    pub unsafe fn new(cstr: *const c_char) -> Option<Self> {
        NonNull::new(cstr as *mut _).map(|cstr| Self {
            cstr,
            phantom: Default::default(),
        })
    }
}

impl<'s> Display for ValidCStr<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(Into::<&str>::into(*self), f)
    }
}

impl<'s> Debug for ValidCStr<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(Into::<&str>::into(*self), f)
    }
}

impl<'l> TryFrom<&'l str> for ValidCStr<'l> {
    type Error = FromBytesWithNulError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        CStr::from_bytes_with_nul(value.as_bytes())?;
        unsafe { Ok(Self::new(value.as_ptr().cast::<c_char>()).unwrap()) }
    }
}

impl<'l> From<ValidCStr<'l>> for *const c_char {
    #[inline]
    fn from(s: ValidCStr<'l>) -> Self {
        s.cstr.as_ptr().cast_const()
    }
}

impl<'l> From<ValidCStr<'l>> for &'l CStr {
    fn from(s: ValidCStr<'l>) -> Self {
        unsafe { CStr::from_ptr(s.cstr.as_ptr()) }
    }
}

impl<'l> From<ValidCStr<'l>> for &'l str {
    fn from(s: ValidCStr<'l>) -> Self {
        unsafe { cstr_to_rust_unchecked(s.cstr.as_ptr()) }
    }
}

/// Owned variant of [`ValidCStr`] which simply wraps a [`CString`].
///
/// It additionally guarantees that the character sequence is valid UTF-8.
pub struct ValidCString(CString);

impl TryFrom<String> for ValidCString {
    type Error = NulError;

    /// Wrapper for [`CString::new`].
    ///
    /// # Example
    /// ```
    /// # use mirabel::ValidCString;
    /// let string : ValidCString = "ValidCString".to_string().try_into().unwrap();
    /// let s : &str = (&string).into();
    /// assert_eq!("ValidCString", s);
    /// ```
    fn try_from(value: String) -> Result<Self, Self::Error> {
        CString::new(value).map(Self)
    }
}

impl<'l> From<&'l ValidCString> for &'l str {
    #[inline]
    fn from(s: &'l ValidCString) -> Self {
        unsafe { from_utf8_unchecked(s.to_bytes()) }
    }
}

impl Deref for ValidCString {
    type Target = CString;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ValidCString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(Into::<&str>::into(self), f)
    }
}

impl Debug for ValidCString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(Into::<&str>::into(self), f)
    }
}
