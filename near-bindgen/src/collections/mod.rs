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
//! # use near_bindgen_macros::near_bindgen;
//!
//! #[near_bindgen]
//! #[derive(Default, BorshDeserialize, BorshSerialize)]
//! pub struct StatusMessage {
//!    records: HashMap<String, String>,
//! }
//! ```
//!
//! The following is an efficient alternative. It will each element individually only when it is
//! read and will save it only when it is written/removed.
//! ```
//! # use borsh::{BorshSerialize, BorshDeserialize};
//! # use near_bindgen_macros::near_bindgen;
//! # use near_bindgen::collections::Map;
//!
//! #[near_bindgen]
//! #[derive(Default, BorshDeserialize, BorshSerialize)]
//! pub struct StatusMessage {
//!    records: Map<String, String>,
//! }
//! ```
//!
//! The efficiency of `Map` comes at the cost, since it has fewer methods than `HashMap` and is not
//! that seemlessly integrated with the rest of the Rust standard library.

mod map;
pub use map::Map;

mod set;
pub use set::Set;

/// Objects stored on the trie directly should have identifiers. If identifier is not provided
/// explicitly than `Default` trait would use this index to generate an id.
pub(crate) static mut NEXT_TRIE_OBJECT_INDEX: u64 = 0;
/// Get next id of the object stored on trie.
pub(crate) fn next_trie_id() -> Vec<u8> {
    unsafe {
        let id = NEXT_TRIE_OBJECT_INDEX;
        NEXT_TRIE_OBJECT_INDEX += 1;
        id.to_le_bytes().to_vec()
    }
}
