//! Testing BorshStorageKey macro.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{near_bindgen, BorshStorageKey};

#[derive(BorshStorageKey, BorshSerialize)]
struct StorageKeyStruct {
    key: String,
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKeyEnum {
    Accounts,
    SubAccounts { account_id: String },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
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

#[near_bindgen]
impl Contract {}

fn main() {}
