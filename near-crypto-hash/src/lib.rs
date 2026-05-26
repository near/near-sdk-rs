/// Raw type for 32 bytes of the hash
pub type CryptoHash = [u8; 32];

mod base58_hash;
pub use base58_hash::{Base58CryptoHash, ParseCryptoHashError};
