//! Main near-sdk types
//!
//! Most types are defined in [`near_sdk_core::types`] and re-exported here for the ease of use.

pub use near_sdk_core::types::*;

// VM types mostly are unused in non-sdk environment, hence, we didn't export them
mod vm_types;
pub use vm_types::*;
