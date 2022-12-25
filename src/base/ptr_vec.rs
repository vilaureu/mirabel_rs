//! [`PtrVec`] is used to return data from plugins.
//!
//! The overhead is minimized by writing data directly into the caller-provided
//! buffer.

use std::{
    fmt::Write,
    mem::{size_of, transmute, MaybeUninit},
    num::NonZeroU8,
    ops::{Deref, DerefMut, Index, IndexMut},
    os::raw::c_char,
    ptr::NonNull,
    slice,
    str::{from_utf8, Utf8Error},
};

/// Vector implementation over a memory buffer with a fixed, run-time capacity.
///
/// [`PtrVec`] allows to perform vector operations on memory not allocated by
/// a [`Vec`].
/// This is especially useful for buffers provided via FFI.
pub struct PtrVec<'l, T> {
    buf: &'l mut [MaybeUninit<T>],
    len: &'l mut usize,
}

impl<'l, T> PtrVec<'l, T> {
    /// Create a [`PtrVec`] which uses the memory at `buf` up to length
    /// `capacity` as storage.
    ///
    /// The length is initially zero and will be stored in `len`.
    ///
    /// # Safety
    /// If `T` is not zero-sized and the `capacity` is not zero, `buf` must
    /// fullfil the requirements of [`slice::from_raw_parts_mut()`].
    #[inline]
    pub(crate) unsafe fn new(mut buf: *mut T, len: &'l mut usize, capacity: usize) -> Self {
        if size_of::<T>() == 0 || capacity == 0 {
            buf = NonNull::dangling().as_ptr();
        }

        *len = 0;
        Self {
            buf: slice::from_raw_parts_mut(buf.cast::<MaybeUninit<T>>(), capacity),
            len,
        }
    }

    /// Length of the vector.
    #[inline]
    pub fn len(&self) -> usize {
        *self.len
    }

    /// Returns whether [`Self::len()`] is zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Length of the underlying buffer and therefore the maximum for
    /// [`Self::len()`].
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Returns whether [`Self::len()`]` == `[`Self::capacity()`].
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity()
    }

    /// Append to the buffer at index [`Self::len()`].
    ///
    /// # Panics
    /// Panics if the vector is full.
    #[inline]
    pub fn push(&mut self, value: T) {
        self.buf
            .get_mut(self.len())
            .expect("cannot push into full PtrVec")
            .write(value);
        *self.len += 1;
    }
}

impl<'l, T: Clone> PtrVec<'l, T> {
    /// Resizes the vector to `new_len`.
    ///
    /// If `new_len` is smaller than the current length, the vector is extended
    /// with clones of `value`.
    ///
    /// # Panics
    /// Panics if `new_len` is larger than the capacity.
    pub fn resize(&mut self, new_len: usize, value: T) {
        assert!(
            new_len <= self.capacity(),
            "cannot resize PtrVec over capacity"
        );

        if new_len <= self.len() {
            for _ in new_len..self.len() {
                *self.len -= 1;
                unsafe {
                    self.buf[self.len()].assume_init_drop();
                }
            }
        } else {
            for _ in self.len()..new_len {
                self.push(value.clone());
            }
        }
    }

    /// Appends all elements of `other` to the vector.
    ///
    /// # Panics
    /// Panics if `other` is larger than the number of free slots.
    pub fn extend_from_slice(&mut self, other: &[T]) {
        assert!(
            other.len() <= self.capacity() - self.len(),
            "not enough free space in PtrVec"
        );

        for value in other.iter() {
            self.push(value.clone());
        }
    }
}

const ACCESS_ERROR: &str = "cannot access element outside PtrVec";

impl<'l, T> Index<usize> for PtrVec<'l, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { self.buf.get(index).expect(ACCESS_ERROR).assume_init_ref() }
    }
}

impl<'l, T> IndexMut<usize> for PtrVec<'l, T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.buf
                .get_mut(index)
                .expect(ACCESS_ERROR)
                .assume_init_mut()
        }
    }
}

impl<'l> PtrVec<'l, NonZeroU8> {
    /// Size must be at least 1.
    #[inline]
    pub(crate) unsafe fn from_c_char(buf: *mut c_char, len: *mut usize, size: usize) -> Self {
        len.write(0);
        Self::new(
            buf.cast(),
            &mut *len,
            size.checked_sub(1)
                .expect("C string buffer must not be of size zero"),
        )
    }
}

impl<'l> Write for PtrVec<'l, NonZeroU8> {
    /// Appends the bytes of `s` to the vector.
    ///
    /// Returns an error when inserting NUL bytes or when the vector is full.
    ///
    /// # Example
    /// ```
    /// # use mirabel::ptr_vec::Storage;
    /// # use std::{ptr::null_mut, fmt::Write};
    /// # #[allow(deref_nullptr)]
    /// # let mut storage = Storage::new(14);
    /// # let mut ptr_vec = storage.get_ptr_vec();
    /// write!(ptr_vec, "example string").expect("failed to write PtrVec");
    /// ```
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for b in s.bytes() {
            let b = NonZeroU8::new(b).ok_or_else(Default::default)?;
            if self.is_full() {
                return Err(Default::default());
            }
            self.push(b);
        }
        Ok(())
    }
}

/// Allocated memory for backing [`PtrVec`]s.
///
/// This is mainly intended for use in tests.
///
/// # Example
/// ```
/// # use mirabel::ptr_vec::Storage;
/// let mut storage = Storage::new(3);
/// let mut ptr_vec = storage.get_ptr_vec();
/// assert!(ptr_vec.is_empty());
/// ptr_vec.push(42);
/// assert_eq!(3, ptr_vec.capacity());
/// assert_eq!(1, ptr_vec.len());
/// assert_eq!([42], *storage);
/// ```
pub struct Storage<T> {
    buf: Vec<MaybeUninit<T>>,
    len: usize,
}

impl<T> Storage<T> {
    /// Create a new [`Storage`] by allocating memory for `capacity` many items.
    pub fn new(capacity: usize) -> Self {
        let mut buf = Vec::with_capacity(capacity);
        buf.resize_with(capacity, || MaybeUninit::uninit());
        Self { buf, len: 0 }
    }

    /// Create a new, empty [`PtrVec`] from the underlying memory.
    ///
    /// The capacity of the new [`PtrVec`] is equal to the capacity of `self`.
    /// The internal storage will be reset.
    #[inline]
    pub fn get_ptr_vec(&mut self) -> PtrVec<T> {
        self.clear();
        PtrVec {
            buf: &mut self.buf,
            len: &mut self.len,
        }
    }

    fn clear(&mut self) {
        unsafe {
            while self.len > 0 {
                self.len -= 1;
                self.buf[self.len].assume_init_drop();
            }
        }
    }
}

impl<T> Drop for Storage<T> {
    fn drop(&mut self) {
        self.clear()
    }
}

impl<T> Deref for Storage<T> {
    type Target = [T];

    /// Returns a slice over the data written by the [`PtrVec`] of
    /// [`Storage::get_ptr_vec()`].
    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { transmute::<&[MaybeUninit<T>], &[T]>(&self.buf[0..self.len]) }
    }
}

impl<T> DerefMut for Storage<T> {
    /// Returns a slice over the data written by the [`PtrVec`] of
    /// [`Storage::get_ptr_vec()`].
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute::<&mut [MaybeUninit<T>], &mut [T]>(&mut self.buf[0..self.len]) }
    }
}

impl Storage<NonZeroU8> {
    /// Tries to convert the initialized bytes to a UTF8 [`str`].
    ///
    /// # Example
    /// ```
    /// # use std::fmt::Write;
    /// # use mirabel::ptr_vec::Storage;
    /// let mut storage = Storage::new(20);
    /// write!(storage.get_ptr_vec(), "Hello World!");
    /// assert_eq!("Hello World!", storage.as_str().expect("UTF8 conversion failed"));
    /// ```
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        from_utf8(unsafe { transmute::<&[NonZeroU8], &[u8]>(self) })
    }
}
