#[cfg(feature = "abi-embed")]
mod abi_embed;
#[cfg(feature = "abi-embed")]
pub use abi_embed::embed;

#[cfg(feature = "abi-generate")]
mod abi_generator;
#[cfg(feature = "abi-generate")]
pub use abi_generator::generate;
