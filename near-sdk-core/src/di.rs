#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
use near_sys as sys;

use crate::types::CryptoHash;

/// Register used internally for atomic operations. This register is safe to use by the user,
/// since it only needs to be untouched while methods of `Environment` execute, which is guaranteed
/// guest code is not parallel.
// TODO: add more const things that we might need to re-export this file fully?
const ATOMIC_OP_REGISTER: u64 = u64::MAX - 2;

#[inline]
#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
pub(crate) unsafe fn read_register_fixed<const N: usize>(register_id: u64) -> [u8; N] {
    let mut buf = [0; N];
    unsafe { sys::read_register(register_id, buf.as_mut_ptr() as _) };
    buf
}

pub struct EnvironmentBasedEnv {}

impl EnvironmentBasedEnv {
    pub fn abort() -> ! {
        // How to adopt this?
        /*
        Use wasm32 unreachable call to avoid including the `panic` external function in Wasm.
        #[cfg(target_arch = "wasm32")]
        This was stabilized recently (~ >1.51), so ignore warnings but don't enforce higher msrv
        #[allow(unused_unsafe)]
        unsafe {
            core::arch::wasm32::unreachable()
        }
        #[cfg(not(target_arch = "wasm32"))]
        unsafe {
            sys::panic()
        }
         */

        #[cfg(target_arch = "wasm32")]
        {
            unsafe { core::arch::wasm32::unreachable() }
        }
        #[cfg(all(not(target_arch = "wasm32"), feature = "__near-sdk-unit-testing", not(test)))]
        {
            unsafe { sys::panic() }
        }
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test),
        ))]
        {
            panic!()
        }
    }

    pub fn panic_str(message: &str) -> ! {
        #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
        {
            unsafe { sys::panic_utf8(message.len() as _, message.as_ptr() as _) }
        }
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test),
        ))]
        {
            eprintln!("{message}");
            panic!()
        }
    }

    pub fn sha256(value: impl AsRef<[u8]>) -> Vec<u8> {
        Self::sha256_array(value).to_vec()
    }

    pub fn keccak256(value: impl AsRef<[u8]>) -> Vec<u8> {
        Self::keccak256_array(value).to_vec()
    }

    pub fn keccak512(value: impl AsRef<[u8]>) -> Vec<u8> {
        Self::keccak512_array(value).to_vec()
    }

    pub fn sha256_array(value: impl AsRef<[u8]>) -> CryptoHash {
        #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
        {
            let value = value.as_ref();
            //* SAFETY: sha256 syscall will always generate 32 bytes inside of the atomic op register
            //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
            //*         because all bytes are filled. This assumes a valid sha256 implementation.
            unsafe {
                sys::sha256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
                read_register_fixed(ATOMIC_OP_REGISTER)
            }
        }
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test),
        ))]
        {
            use sha2::Digest;

            sha2::Sha256::digest(value).into()
        }
    }

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

        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test),
        ))]
        {
            use sha3::Digest;

            sha3::Keccak256::digest(value).into()
        }
    }

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
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test),
        ))]
        {
            use sha3::Digest;

            sha3::Keccak512::digest(value).into()
        }
    }

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
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test),
        ))]
        {
            use sha2::Digest;

            ripemd::Ripemd160::digest(value).into()
        }
    }
}
