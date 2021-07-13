//! Helper types for JSON serialization.

mod hash;
mod integers;
mod vector;

pub use hash::Base58CryptoHash;
pub use integers::{I128, I64, U128, U64};
pub use vector::Base64VecU8;
use crate::types::PublicKey;

#[deprecated(
    since = "4.0.0",
    note = "PublicKey type is now unified with Base58PublicKey. It is \
            recommended to use PublicKey going forward to avoid using \
            similar sounding types for the same thing."
)]
pub type Base58PublicKey = PublicKey;
