//! Low-level abstraction over `near-sys` host functions. Provides the same
//! hashing (and related) API on wasm32 (via NEAR VM host calls) and on
//! non-wasm32 (via pure-Rust crates), so off-chain code can compute identically
//! to on-chain.

mod hash;

pub use hash::*;

pub type CryptoHash = [u8; 32];

#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
use near_sys as sys;

#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
/// Register used internally for atomic operations. This register is safe to use by the user,
/// since it only needs to be untouched while methods of `Environment` execute, which is guaranteed
/// guest code is not parallel.
pub(crate) const ATOMIC_OP_REGISTER: u64 = u64::MAX - 2;

#[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
#[inline]
pub(crate) unsafe fn read_register_fixed<const N: usize>(register_id: u64) -> [u8; N] {
    let mut buf = [0; N];
    unsafe { sys::read_register(register_id, buf.as_mut_ptr() as _) };
    buf
}

pub fn abort() -> ! {
    #[cfg(target_arch = "wasm32")]
    {
        //* This was stabilized recently (~ >1.51), so ignore warnings but don't enforce higher msrv
        #[allow(unused_unsafe)]
        unsafe {
            core::arch::wasm32::unreachable()
        }
    }
    #[cfg(all(not(target_arch = "wasm32"), feature = "__near-sdk-unit-testing", not(test)))]
    {
        unsafe { sys::panic() }
    }
    #[cfg(all(not(target_arch = "wasm32"), any(not(feature = "__near-sdk-unit-testing"), test),))]
    {
        panic!()
    }
}

pub fn panic_str(message: &str) -> ! {
    #[cfg(any(target_arch = "wasm32", all(feature = "__near-sdk-unit-testing", not(test))))]
    {
        unsafe { sys::panic_utf8(message.len() as _, message.as_ptr() as _) }
    }
    #[cfg(all(not(target_arch = "wasm32"), any(not(feature = "__near-sdk-unit-testing"), test)))]
    {
        eprintln!("{message}");
        panic!()
    }
}
