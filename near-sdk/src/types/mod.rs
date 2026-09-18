//! Main near-sdk types
//!
//! Most types are defined in [`near_sdk_core::types`] and re-exported here for the ease of use.

pub use near_sdk_core::types::*;

// `near-sdk-core` only re-exports the generic account id types; the typed `0u` id arrived in
// `near-account-id` 3.1 and is re-exported here so contracts can name it as `near_sdk::UniversalAccountId`.
pub use near_account_id::{ParseUniversalAccountIdError, UniversalAccountId};

// NOTE: VM types are mostly unused outside `near-sdk`, hence, they are not exposed by
// `near-sdk-core`
mod vm_types;
pub use vm_types::*;
