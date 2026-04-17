use crate::CryptoHash;

#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
use near_sys as sys;

#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
use crate::{ATOMIC_OP_REGISTER, read_register_fixed};

/// Hashes the random sequence of bytes using sha256.
///
/// # Examples
/// ```
/// use near_env::sha256;
/// use hex;
///
/// assert_eq!(
///     sha256(b"The phrase that will be hashed"),
///     hex::decode("7fc38bc74a0d0e592d2b8381839adc2649007d5bca11f92eeddef78681b4e3a3").expect("Decoding failed")
/// );
/// ```
pub fn sha256(value: impl AsRef<[u8]>) -> Vec<u8> {
    sha256_array(value.as_ref()).to_vec()
}

/// Hashes the random sequence of bytes using keccak256.
///
/// # Examples
/// ```
/// use near_env::keccak256;
/// use hex;
///
/// assert_eq!(
///     keccak256(b"The phrase that will be hashed"),
///     hex::decode("b244af9dd4aada2eda59130bbcff112f29b427d924b654aaeb5a0384fa9afed4")
///         .expect("Decoding failed")
/// );
/// ```
pub fn keccak256(value: impl AsRef<[u8]>) -> Vec<u8> {
    keccak256_array(value.as_ref()).to_vec()
}

/// Hashes the random sequence of bytes using keccak512.
///
/// # Examples
/// ```
/// use near_env::keccak512;
/// use hex;
///
/// assert_eq!(
///     keccak512(b"The phrase that will be hashed"),
///     hex::decode("29a7df7b889a443fdfbd769adb57ef7e98e6159187b582baba778c06e8b41a75f61367257e8c525a95b3f13ddf432f115d1df128a910c8fc93221db136d92b31")
///         .expect("Decoding failed")
/// );
/// ```
pub fn keccak512(value: impl AsRef<[u8]>) -> Vec<u8> {
    keccak512_array(value.as_ref()).to_vec()
}

/// Hashes the bytes using the SHA-256 hash function. This returns a 32 byte hash.
///
/// # Examples
/// ```
/// use near_env::sha256_array;
/// use hex;
///
/// assert_eq!(
///     &sha256_array(b"The phrase that will be hashed"),
///     hex::decode("7fc38bc74a0d0e592d2b8381839adc2649007d5bca11f92eeddef78681b4e3a3")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn sha256_array(value: impl AsRef<[u8]>) -> CryptoHash {
    #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
    {
        let value = value.as_ref();
        unsafe {
            sys::sha256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
            read_register_fixed(ATOMIC_OP_REGISTER)
        }
    }
    #[cfg(all(not(target_arch = "wasm32"), any(not(feature = "__near-sdk-unit-testing"), test)))]
    {
        use sha2::Digest;

        sha2::Sha256::digest(value).into()
    }
}

/// Hashes the bytes using the Keccak-256 hash function. This returns a 32 byte hash.
///
/// # Examples
/// ```
/// use near_env::keccak256_array;
/// use hex;
///
/// assert_eq!(
///     &keccak256_array(b"The phrase that will be hashed"),
///     hex::decode("b244af9dd4aada2eda59130bbcff112f29b427d924b654aaeb5a0384fa9afed4")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn keccak256_array(value: impl AsRef<[u8]>) -> CryptoHash {
    #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
    {
        let value = value.as_ref();
        //* SAFETY: keccak256 syscall will always generate 32 bytes inside of the atomic op register
        //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
        //*         because all bytes are filled. This assumes a valid keccak256 implementation.
        unsafe {
            sys::keccak256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
            read_register_fixed(ATOMIC_OP_REGISTER)
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), any(not(feature = "__near-sdk-unit-testing"), test)))]
    {
        use sha3::Digest;

        sha3::Keccak256::digest(value).into()
    }
}

/// Hashes the bytes using the Keccak-512 hash function. This returns a 64 byte hash.
///
/// # Examples
/// ```
/// use near_env::keccak512_array;
/// use hex;
///
/// assert_eq!(
///     &keccak512_array(b"The phrase that will be hashed"),
///     hex::decode("29a7df7b889a443fdfbd769adb57ef7e98e6159187b582baba778c06e8b41a75f61367257e8c525a95b3f13ddf432f115d1df128a910c8fc93221db136d92b31")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn keccak512_array(value: impl AsRef<[u8]>) -> [u8; 64] {
    #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
    {
        let value = value.as_ref();

        //* SAFETY: keccak512 syscall will always generate 64 bytes inside of the atomic op register
        //*         so the read will have a sufficient buffer of 64, and can transmute from uninit
        //*         because all bytes are filled. This assumes a valid keccak512 implementation.
        unsafe {
            sys::keccak512(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
            read_register_fixed(ATOMIC_OP_REGISTER)
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), any(not(feature = "__near-sdk-unit-testing"), test)))]
    {
        use sha3::Digest;

        sha3::Keccak512::digest(value).into()
    }
}

/// Hashes the bytes using the RIPEMD-160 hash function. This returns a 20 byte hash.
///
/// # Examples
/// ```
/// use near_env::ripemd160_array;
/// use hex;
///
/// assert_eq!(
///     &ripemd160_array(b"The phrase that will be hashed"),
///     hex::decode("9a48b9195fcb14cfe6051c0a1be7882efcadaed8")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn ripemd160_array(value: impl AsRef<[u8]>) -> [u8; 20] {
    #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
    {
        let value = value.as_ref();
        //* SAFETY: ripemd160 syscall will always generate 20 bytes inside of the atomic op register
        //*         so the read will have a sufficient buffer of 20, and can transmute from uninit
        //*         because all bytes are filled. This assumes a valid ripemd160 implementation.
        unsafe {
            sys::ripemd160(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
            read_register_fixed(ATOMIC_OP_REGISTER)
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), any(not(feature = "__near-sdk-unit-testing"), test)))]
    {
        use sha2::Digest;

        ripemd::Ripemd160::digest(value).into()
    }
}
