//! Helper crate to automatically chose a backend for digest implementations.
//! Currently supported backends are:
//!
//! * `cfg(near)`: via Near host-function
//! * default: fallback to pure Rust implementation

pub use digest::*;

#[cfg(feature = "ripemd")]
pub mod ripemd;
#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "sha3")]
pub mod sha3;
#[cfg(near)]
mod utils;

// TODO: use `cfg_check!` macro to reduce duplicates once we reach rustc 1.95

macro_rules! digest_cfg {
    ($vis:vis struct $name:ident {
        near => $near_path:path,
        local => $local_path:path $(,)?
    }) => {
        #[cfg(near)]
        #[derive(Debug, Clone, Default)]
        #[repr(transparent)]
        $vis struct $name($near_path);

        #[cfg(not(near))]
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
}
pub(crate) use digest_cfg;
