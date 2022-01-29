use borsh::BorshSerialize;

use crate::env;

mod private {
    /// Seal `ToKey` implementations to limit usage to the builtin implementations
    pub trait Sealed {}

    impl Sealed for super::Sha256 {}
    impl Sealed for super::Keccak256 {}
    impl Sealed for super::Identity {}
}

/// Trait used to generate keys to store data based on a serializable structure.
pub trait ToKey: self::private::Sealed {
    /// Output type for the generated lookup key.
    type KeyType: AsRef<[u8]>;

    fn to_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize;
}

/// Sha256 hash helper which hashes through a syscall. This type satisfies the [`ToKey`] trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Sha256 {}

impl ToKey for Sha256 {
    type KeyType = [u8; 32];

    fn to_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes, then hash the combined value.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        env::sha256_array(buffer)
    }
}

/// Keccak256 hash helper which hashes through a syscall. This type satisfies the [`ToKey`] trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keccak256 {}

impl ToKey for Keccak256 {
    type KeyType = [u8; 32];

    fn to_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes, then hash the combined value.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        env::keccak256_array(buffer)
    }
}

/// Identity hash which just prefixes all of the serializes bytes and uses it as the key.
pub enum Identity {}

impl ToKey for Identity {
    type KeyType = Vec<u8>;

    fn to_key<Q: ?Sized>(prefix: &[u8], key: &Q, buffer: &mut Vec<u8>) -> Self::KeyType
    where
        Q: BorshSerialize,
    {
        // Prefix the serialized bytes and return a copy of this buffer.
        buffer.extend(prefix);
        key.serialize(buffer).unwrap_or_else(|_| env::abort());

        buffer.clone()
    }
}
