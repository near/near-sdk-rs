//! Length-bounded wrappers around [`String`] and [`Vec`].
//!
//! Contract inputs are untrusted. A plain `String` or `Vec<T>` field accepts arbitrarily large
//! values, which inflate storage and gas costs or grow hashed data without bound. The types in
//! this module enforce a compile-time maximum on construction and on every deserialization path
//! they support, so an oversized value is rejected before it reaches contract logic:
//!
//! - [`BoundedString`]: UTF-8 string, JSON string or Borsh `String`.
//! - [`BoundedVec`]: vector of any `T`, Borsh `Vec<T>` only. It serializes to JSON but does not
//!   deserialize from it.
//! - [`BoundedBase64VecU8`]: bytes, JSON base64 string or Borsh `Vec<u8>`. Use it for byte
//!   arguments passed as JSON.
//!
//! `serde` gives no way to stop inside a JSON string token: the deserializer scans the whole
//! string before the type sees it. So rejecting an oversized JSON value costs the same as parsing
//! it into the unbounded type, and the bound saves the copy, decoding and storage after that.
//! `BoundedVec` does not deserialize from JSON. A JSON array can be rejected after `MAX + 1`
//! elements, so its rejection cost would differ from the string types by up to a scan of the
//! whole input. Bytes go through [`BoundedBase64VecU8`] instead, a string like the others.

mod base64_vec;
mod string;
mod vec;

pub use self::base64_vec::BoundedBase64VecU8;
pub use self::string::BoundedString;
pub use self::vec::BoundedVec;

use std::fmt;

/// Error returned when a value does not fit into a [`BoundedString`], [`BoundedVec`] or
/// [`BoundedBase64VecU8`].
///
/// `len` is the length of the rejected value and `max` the bound it violated. For
/// [`BoundedString`] both are measured in UTF-8 bytes; for [`BoundedVec`] in elements; for
/// [`BoundedBase64VecU8`] in decoded bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TooLongError {
    /// Length of the rejected value.
    pub len: usize,
    /// Maximum length allowed by the bound.
    pub max: usize,
}

impl fmt::Display for TooLongError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERR_TOO_LONG: length {} exceeds the maximum of {}", self.len, self.max)
    }
}

impl std::error::Error for TooLongError {}

#[cfg(feature = "borsh")]
impl From<TooLongError> for std::io::Error {
    fn from(e: TooLongError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    }
}

/// Helper that compares provided string length and max length. Returns `TooLongError` if len > max.
#[inline]
fn check_len(len: usize, max: usize) -> Result<(), TooLongError> {
    if len > max { Err(TooLongError { len, max }) } else { Ok(()) }
}
