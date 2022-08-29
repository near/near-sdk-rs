#[cfg(feature = "__abi-embed")]
mod abi_embed;
#[cfg(feature = "__abi-embed")]
pub use abi_embed::embed;

#[cfg(feature = "__abi-generate")]
mod abi_generator;
#[cfg(feature = "__abi-generate")]
pub use abi_generator::generate;
