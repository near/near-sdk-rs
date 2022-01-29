use crate::env;

mod private {
    /// Seal `CryptoHasher` implementations to limit usage to the builtin implementations
    pub trait Sealed {}

    impl Sealed for super::Sha256 {}
    impl Sealed for super::Keccak256 {}
}

/// Cryptographic hashes that can be used within the SDK as a hashing function.
pub trait CryptoHasher: self::private::Sealed {
    /// Output type of the hashing function.
    type Digest;

    /// Hashes raw bytes and returns the `Digest` output.
    fn hash(ingest: &[u8]) -> Self::Digest;
}

/// Sha256 hash helper which hashes through a syscall. This type satisfies the [`CryptoHasher`]
/// trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Sha256 {}

impl CryptoHasher for Sha256 {
    type Digest = [u8; 32];

    fn hash(ingest: &[u8]) -> Self::Digest {
        env::sha256_array(ingest)
    }
}

/// Keccak256 hash helper which hashes through a syscall. This type satisfies the [`CryptoHasher`]
/// trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keccak256 {}

impl CryptoHasher for Keccak256 {
    type Digest = [u8; 32];

    fn hash(ingest: &[u8]) -> Self::Digest {
        env::keccak256_array(ingest)
    }
}
