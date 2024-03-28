//! Check types from near_sdk.

use near_sdk::near;
use near_sdk::collections::{LookupMap, LookupSet, TreeMap, UnorderedMap, UnorderedSet, Vector};
use near_sdk::json_types::Base58CryptoHash;
use near_sdk::store::{Lazy, LazyOption};
use near_sdk::CurveType;

#[near(contract_state)]
struct TypesContainer {
    lookup_map: LookupMap<u32, u64>,
    lookup_set: LookupSet<u32>,
    tree_map: TreeMap<u32, u64>,
    unordered_map: UnorderedMap<u32, u64>,
    unordered_set: UnorderedSet<u32>,
    vector: Vector<u32>,
    base58_crypto_hash: Base58CryptoHash,
    u64_type: near_sdk::json_types::U64,
    base64_vec_u8: near_sdk::json_types::Base64VecU8,
    lazy: Lazy<u64>,
    lazy_option: LazyOption<u64>,
    curve_type: CurveType,
}

fn main() {}