//! Main near-sdk types
//!
//! Most types are defined in [`near_sdk_core::types`] and re-exported here for the ease of use.

pub use near_sdk_core::types::*;

// NOTE: VM types are mostly unused outside `near-sdk`, hence, they are not exposed by
// `near-sdk-core`
mod vm_types;
pub use vm_types::*;
