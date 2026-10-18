//! Claim 4 with a realistic shape: a near-contract-standards `FungibleToken` (which holds a
//! `collections::LookupMap`) as a field of a `#[near(storage)]` record. The derive sees an
//! opaque struct name, accepts it, the record is blanket-`Removable`, and `remove` orphans
//! the balances. And `use near_sdk::store::LookupMap;` next to `use near_sdk::state::*;`
//! silently makes every `LookupMap` in the file the v1 type (explicit imports shadow globs).

use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::state::testing;
use near_sdk::state::{ContractState, Map};
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::{AccountId, env, near, testing_env};

#[near(storage)]
pub struct Pool {
    #[key(1, required)]
    pub token: FungibleToken,
}

#[near(contract_state(layout = v2))]
pub struct Pools {
    #[key(1)]
    pub pools: Map<u32, Pool>,
}

mod shadowed {
    #![allow(unused_imports, dead_code)]
    use near_sdk::near;
    use near_sdk::state::*;
    use near_sdk::store::LookupMap; // shadows `state::LookupMap` from the glob, silently

    #[near(storage)]
    pub struct Profile {
        // The author means the v2 `LookupMap`; this is `store::LookupMap` and nothing says so.
        #[key(1, required)]
        pub notes: LookupMap<u32, u32>,
    }
    pub fn is_v1(_: &near_sdk::store::LookupMap<u32, u32>) {}
    pub fn check(p: &Profile) {
        is_v1(&p.notes)
    }
}

fn ctx() {
    testing_env!(VMContextBuilder::new().storage_usage(env::storage_usage()).build());
}

#[test]
fn removing_a_record_that_holds_a_fungible_token_orphans_its_balances() {
    ctx();
    testing::use_env_host();
    let c = Pools { pools: Default::default() };
    c.state_write();
    drop(c);

    testing::begin_call();
    let mut c = Pools::state_read().unwrap();
    let alice: AccountId = "alice.near".parse().unwrap();
    let mut token = FungibleToken::new(b"p1".to_vec());
    token.internal_register_account(&alice);
    token.internal_deposit(&alice, 500);
    let _ = c.pools.insert(1, Pool { token });
    c.state_write();
    drop(c);
    let v1_key = [b"p1".as_slice(), &near_sdk::borsh::to_vec(&alice).unwrap()].concat();
    assert!(env::storage_has_key(&v1_key));

    testing::begin_call();
    let mut c = Pools::state_read().unwrap();
    assert!(c.pools.remove(&1).is_some(), "compiles and runs: Pool is Removable");
    c.state_write();
    drop(c);
    assert!(env::storage_has_key(&v1_key), "orphaned: the pool record is gone, alice's balance is not");
    let left: Vec<_> = testing::env_store().into_keys().filter(|k| k.starts_with(b"p1")).collect();
    assert_eq!(left.len(), 1);
}
