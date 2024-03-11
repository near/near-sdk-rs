//! Testing BorshStorageKey macro.

use borsh::BorshSerialize;
use near_sdk::near;
use near_sdk::collections::LookupMap;
use near_sdk::BorshStorageKey;

#[derive(BorshStorageKey, BorshSerialize)]
struct StorageKeyStruct {
    key: String,
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKeyEnum {
    Accounts,
    SubAccounts { account_id: String },
}

#[near(contract_state)]
struct Contract {
    map1: LookupMap<u64, u64>,
    map2: LookupMap<String, String>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            map1: LookupMap::new(StorageKeyStruct { key: "bla".to_string() }),
            map2: LookupMap::new(StorageKeyEnum::Accounts),
        }
    }
}

#[near]
impl Contract {}

fn main() {}
