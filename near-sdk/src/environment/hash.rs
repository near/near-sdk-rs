use std::mem::MaybeUninit;

use crate::sys;

const ATOMIC_OP_REGISTER: u64 = u64::MAX - 2;

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
        //* SAFETY: sha256 syscall will always generate 32 bytes inside of the atomic op register
        //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
        //*         because all bytes are filled. This assumes a valid sha256 implementation.
        unsafe {
            sys::sha256(ingest.len() as _, ingest.as_ptr() as _, ATOMIC_OP_REGISTER);

            let mut hash = [MaybeUninit::<u8>::uninit(); 32];
            sys::read_register(ATOMIC_OP_REGISTER, hash.as_mut_ptr() as _);
            std::mem::transmute(hash)
        }
    }
}

/// Keccak256 hash helper which hashes through a syscall. This type satisfies the [`CryptoHasher`]
/// trait.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keccak256 {}

impl CryptoHasher for Keccak256 {
    type Digest = [u8; 32];

    fn hash(ingest: &[u8]) -> Self::Digest {
        //* SAFETY: keccak256 syscall will always generate 32 bytes inside of the atomic op register
        //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
        //*         because all bytes are filled. This assumes a valid keccak256 implementation.
        unsafe {
            sys::keccak256(ingest.len() as _, ingest.as_ptr() as _, ATOMIC_OP_REGISTER);

            let mut hash = [MaybeUninit::<u8>::uninit(); 32];
            sys::read_register(ATOMIC_OP_REGISTER, hash.as_mut_ptr() as _);
            std::mem::transmute(hash)
        }
    }
}
