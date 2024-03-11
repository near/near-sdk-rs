//! Testing BorshStorageKey macro with lifetimes and generics.

use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::BorshStorageKey;
use near_sdk::near;

#[derive(BorshStorageKey, BorshSerialize)]
struct StorageKeyStruct<'a, T>
where
    T: ?Sized,
{
    key: &'a T,
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKeyEnum<'a, T>
where
    T: ?Sized,
{
    Accounts,
    SubAccounts { account_id: &'a T },
}

#[near(contract_state)]
struct Contract {
    map1: LookupMap<u64, u64>,
    map2: LookupMap<String, String>,
}

impl Default for Contract {
    fn default() -> Self {
        let a = "test".to_string();
        Self {
            map1: LookupMap::new(StorageKeyStruct { key: "bla" }),
            map2: LookupMap::new(StorageKeyEnum::SubAccounts { account_id: &a }),
        }
    }
}

#[near]
impl Contract {}

fn main() {}
