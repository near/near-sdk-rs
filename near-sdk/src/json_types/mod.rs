//! Helper types for JSON serialization.

mod account;
mod hash;
mod integers;
mod public_key;
mod vector;

pub use account::ValidAccountId;
pub use hash::Base58CryptoHash;
pub use integers::{I128, I64, U128, U64};
pub use public_key::{Base58PublicKey, CurveType};
pub use vector::Base64VecU8;

/// Duration in nanosecond wrapped into a struct for JSON serialization as a string.
pub type WrappedDuration = U64;

/// Timestamp in nanosecond wrapped into a struct for JSON serialization as a string.
pub type WrappedTimestamp = U64;

/// Balance wrapped into a struct for JSON serialization as a string.
pub type WrappedBalance = U128;
