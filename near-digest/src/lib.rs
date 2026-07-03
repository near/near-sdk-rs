//! Cryptographic hash functions that automatically choose the backend for the compilation
//! target.
//!
//! Every hash type in this crate is a thin `#[repr(transparent)]` wrapper that selects
//! its implementation at compile time:
//!
//! * `cfg(near)` — the crate is compiled as a NEAR contract (the cfg is set by
//!   `cargo near build`): hashing is delegated to the corresponding NEAR host function,
//!   which is significantly cheaper in gas than computing the hash inside the Wasm
//!   module.
//! * everything else — native binaries, unit tests, tooling, non-contract Wasm:
//!   falls back to the pure-Rust [RustCrypto](https://github.com/RustCrypto/hashes)
//!   implementation of the same algorithm.
//!
//! Both backends produce identical output for identical input, so code using these
//! types behaves the same on-chain and off-chain.
//!
//! All types implement the traits of the [`digest`] crate ([`Digest`](digest::Digest),
//! [`Update`](digest::Update), [`FixedOutput`](digest::FixedOutput), ...), which makes
//! them drop-in replacements for the RustCrypto types in any generic API that accepts
//! a `D: Digest` parameter.
//!
//! # Example
//!
//! ```
//! use near_digest::digest::Digest;
//! use near_digest::sha2::Sha256;
//!
//! let hash = Sha256::digest(b"near is cool!");
//!
//! // or incrementally:
//! let mut hasher = Sha256::new();
//! hasher.update(b"near ");
//! hasher.update(b"is cool!");
//! assert_eq!(hasher.finalize(), hash);
//! ```
//!
//! # Available hashes and feature flags
//!
//! No features are enabled by default; each hash family is gated behind its own
//! feature:
//!
//! | Type | Feature | NEAR host function |
//! |------|---------|--------------------|
//! | `sha2::Sha256` | `sha2` | `sha256_array` |
//! | `sha3::Keccak256` | `sha3` | `keccak256_array` |
//! | `sha3::Keccak512` | `sha3` | `keccak512_array` |
//! | `sha3::Sha3_256` | `sha3` + `unstable` | - (pure Rust on all targets for now) |
//! | `sha3::Sha3_512` | `sha3` + `unstable` | - (pure Rust on all targets for now) |
//! | `ripemd::Ripemd160` | `ripemd` | `ripemd160_array` |
//!
//! Additional features:
//!
//! * `zeroize` — implements `zeroize::ZeroizeOnDrop` for all hash types, clearing
//!   buffered input from memory when a hasher is dropped.
//! * `unstable` — enables items whose API or backend may change in a breaking way
//!   between minor releases.
//!
//! # On-chain buffering
//!
//! The NEAR host functions are one-shot: they take the whole message and return the
//! hash. To still support the [`Update`](digest::Update) API, the `cfg(near)`
//! backend buffers all input in memory and invokes the host function once at
//! finalization.
//!

// re-export of the `digest` crate
pub use digest;

#[cfg(feature = "ripemd")]
pub mod ripemd;
#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "sha3")]
pub mod sha3;
#[cfg(all(near, any(feature = "ripemd", feature = "sha2", feature = "sha3")))]
mod utils;

// TODO: use `cfg_check!` macro to reduce duplicates once we reach rustc 1.95

/// Defines a hash type that wraps `$near_path` under `cfg(near)` and `$local_path` otherwise,
/// forwarding the `digest` traits to whichever backend was selected. The `near =>` arm is optional
/// for hashes that have no host-function counterpart.
#[cfg(any(feature = "ripemd", feature = "sha2", feature = "sha3"))]
macro_rules! digest_cfg {
    ($(#[$attr:meta])* $vis:vis struct $name:ident {
        near => $near_path:path,
        _ => $local_path:path $(,)?
    }) => {
        #[cfg(near)]
        $(#[$attr])*
        #[derive(Debug, Clone, Default)]
        #[repr(transparent)]
        $vis struct $name($near_path);

        #[cfg(not(near))]
        $(#[$attr])*
        #[derive(Debug, Clone, Default)]
        #[repr(transparent)]
        $vis struct $name($local_path);

        #[cfg(near)]
        impl ::digest::OutputSizeUser for $name {
            type OutputSize = <$near_path as ::digest::OutputSizeUser>::OutputSize;
        }

        #[cfg(not(near))]
        impl ::digest::OutputSizeUser for $name {
            type OutputSize = <$local_path as ::digest::OutputSizeUser>::OutputSize;
        }

        impl ::digest::Update for $name {
            #[inline]
            fn update(&mut self, data: &[u8]) {
                ::digest::Update::update(&mut self.0, data);
            }
        }

        impl ::digest::FixedOutput for $name {
            #[inline]
            fn finalize_into(self, out: &mut ::digest::Output<Self>) {
                ::digest::FixedOutput::finalize_into(self.0, out);
            }
        }

        impl ::digest::HashMarker for $name {}

        #[cfg(feature = "zeroize")]
        impl ::zeroize::ZeroizeOnDrop for $name {}
    };

    ($(#[$attr:meta])* $vis:vis struct $name:ident {
        _ => $local_path:path $(,)?
    }) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Default)]
        #[repr(transparent)]
        $vis struct $name($local_path);

        impl ::digest::OutputSizeUser for $name {
            type OutputSize = <$local_path as ::digest::OutputSizeUser>::OutputSize;
        }

        impl ::digest::Update for $name {
            #[inline]
            fn update(&mut self, data: &[u8]) {
                ::digest::Update::update(&mut self.0, data);
            }
        }

        impl ::digest::FixedOutput for $name {
            #[inline]
            fn finalize_into(self, out: &mut ::digest::Output<Self>) {
                ::digest::FixedOutput::finalize_into(self.0, out);
            }
        }

        impl ::digest::HashMarker for $name {}

        #[cfg(feature = "zeroize")]
        impl ::zeroize::ZeroizeOnDrop for $name {}
    }
}
#[cfg(any(feature = "ripemd", feature = "sha2", feature = "sha3"))]
pub(crate) use digest_cfg;
