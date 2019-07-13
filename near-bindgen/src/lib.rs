#![feature(const_vec_new)]
#[macro_use]
extern crate near_bindgen_macros;
pub use near_bindgen_macros::near_bindgen;

mod binding;
pub use crate::binding::*;

mod header;
pub use crate::header::*;

pub mod collections;

pub mod blockchain;

/// Objects stored on the trie directly should have identifiers. If identifier is not provided
/// explicitly than `Default` trait would use this index to generate an id.
pub(crate) static mut NEXT_TRIE_OBJECT_INDEX: usize = 0;
/// Get next id of the object stored on trie.
pub(crate) fn next_trie_id() -> String {
    unsafe {
        let id = NEXT_TRIE_OBJECT_INDEX;
        NEXT_TRIE_OBJECT_INDEX += 1;
        format!("id{}", id)
    }
}
