#[cfg(any(feature = "__abi-embed", feature = "__abi-generate"))]
pub(crate) mod abi;
mod code_generator;
mod info_extractor;
mod metadata;
mod utils;
pub(crate) use code_generator::*;
pub(crate) use info_extractor::*;
pub(crate) use metadata::metadata_visitor::MetadataVisitor;
