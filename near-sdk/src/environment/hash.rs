use borsh::BorshSerialize;

use crate::env;

mod private {
    /// Seal `CryptoHasher` implementations to limit usage to the builtin implementations
    pub trait Sealed {}

    impl Sealed for super::Sha256 {}
    impl Sealed for super::Keccak256 {}
    impl Sealed for super::Identity {}
}

/// Trait used to generate keys to store data based on a serializable structure.
pub trait StorageKeyer: self::private::Sealed {
    /// Output type for the generated lookup key.
    type KeyType;

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize;
}

/// Sha256 hash helper which hashes through a syscall. This type satisfies the [`CryptoHasher`]
/// trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Sha256 {}

impl StorageKeyer for Sha256 {
    type KeyType = [u8; 32];

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes, then hash the combined value.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        env::sha256_array(buffer)
    }
}

/// Keccak256 hash helper which hashes through a syscall. This type satisfies the [`CryptoHasher`]
/// trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keccak256 {}

impl StorageKeyer for Keccak256 {
    type KeyType = [u8; 32];

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes, then hash the combined value.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        env::keccak256_array(buffer)
    }
}

pub enum Identity {}

impl StorageKeyer for Identity {
    type KeyType = Vec<u8>;

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes and return a copy of this buffer.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        buffer.clone()
    }
}
