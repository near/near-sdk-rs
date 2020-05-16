use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{UnorderedMap, Heap};

/// HeapMap allows iterating over keys and entries based on natural key ordering.
///
/// Internals:
/// - `indices`: map `K -> u64` containing key's index in sorted order, required to ensure O(log(N))
///   runtime complexity of `remove` in worst case (avoid linear scan to find sorted index for key)
/// - `keys`: min-heap of keys
///   - remove/insert return swaps performed to maintain heap order, `indices` must be updated
///     accordingly to keep the state consistent
/// - `iter()`: iterate over sorted order of keys with O(Nlog(N)) runtime complexity - effectively
///   in-place heap-sort is performed, so all swaps must be mirrored to `indices`
///
/// Runtime complexity (worst case):
///   - `contains_key`: O(1)
///   - `get`: O(1)
///   - `insert`: O(log(N))
///   - `remove`: O(log(N))
///   - `keys` (iterator): O(Nlog(N))
///   - `entries` (iterator): O(Nlog(N))
#[derive(BorshSerialize, BorshDeserialize)]
pub struct HeapMap<K, V> {
    key_index_prefix: Vec<u8>,
    keys: Heap<K>,
    indices: UnorderedMap<K, u64>,
    _values: PhantomData<V>,
}

impl<K, V> HeapMap<K, V>
    where
        K: Ord + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
{
    // TODO new
    // TODO len
    // TODO get
    // TODO contains_key
    // TODO remove: update indices after maintaining heap order
    // TODO insert: update indices after maintaining heap order
    // TODO clear
    // TODO keys (iterator): update indices for sorted order
    // TODO entries (iterator): update indices for sorted order
}
