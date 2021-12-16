use borsh::BorshSerialize;

use crate::env;

mod private {
    /// Seal `CryptoHasher` implementations to limit usage to the builtin implementations
    pub trait Sealed {}

    impl Sealed for super::Sha256 {}
    impl Sealed for super::Keccak256 {}
    impl Sealed for super::Identity {}
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

pub trait StorageKeyer: self::private::Sealed {
    type KeyType;

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize;
}

impl<T> StorageKeyer for T
where
    T: CryptoHasher,
{
    type KeyType = <T as CryptoHasher>::Digest;

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Concat the prefix with serialized key and hash the bytes for the lookup key.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        T::hash(buffer)
    }
}

pub enum Identity {}

impl StorageKeyer for Identity {
    type KeyType = Vec<u8>;

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        buffer.clone()
    }
}
