//! Collections that offer an alternative to standard containers from `std::collections::*` by
//! utilizing the underlying blockchain trie storage more efficiently.
//!
//! For example, the following smart contract does not work with state efficiently, because it will
//! load the entire `HashMap` at the beginning of the contract call, and will save it entirely at
//! the end, in cases when there is state modification. This is fine for small number of elements,
//! but very inefficient for large numbers.
//!
//! ```
//! # use std::collections::HashMap;
//! # use borsh::{BorshSerialize, BorshDeserialize};
//! # use near_sdk_macros::near_bindgen;
//!
//! #[near_bindgen]
//! #[derive(BorshDeserialize, BorshSerialize)]
//! pub struct StatusMessage {
//!    records: HashMap<String, String>,
//! }
//! ```
//!
//! The following is an efficient alternative. It will each element individually only when it is
//! read and will save it only when it is written/removed.
//! ```
//! # use borsh::{BorshSerialize, BorshDeserialize};
//! # use near_sdk_macros::near_bindgen;
//! # use near_sdk::collections::LookupMap;
//!
//! #[near_bindgen]
//! #[derive(BorshDeserialize, BorshSerialize)]
//! pub struct StatusMessage {
//!    records: LookupMap<String, String>,
//! }
//! ```
//!
//! The efficiency of `LookupMap` comes at the cost, since it has fewer methods than `HashMap` and is not
//! that seemlessly integrated with the rest of the Rust standard library.

mod legacy_tree_map;
pub use legacy_tree_map::LegacyTreeMap;

mod lookup_map;
pub use lookup_map::LookupMap;

mod lookup_set;
pub use lookup_set::LookupSet;

mod vector;
pub use vector::Vector;

mod unordered_map;
pub use unordered_map::UnorderedMap;

mod unordered_set;
pub use unordered_set::UnorderedSet;

mod tree_map;
pub use tree_map::TreeMap;

pub const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
pub const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element with Borsh.";
pub const ERR_ELEMENT_DESERIALIZATION: &[u8] = b"Cannot deserialize element with Borsh.";

pub(crate) fn append(id: &[u8], chr: u8) -> Vec<u8> {
    append_slice(id, &[chr])
}

pub(crate) fn append_slice(id: &[u8], extra: &[u8]) -> Vec<u8> {
    [id, extra].concat()
}
