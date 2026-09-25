//! Universal state init (NEP-655) through the mocked host functions.
//!
//! An integration test links near-sdk without `cfg(test)`, so with `unit-testing` every `env` call
//! here goes through nearcore's host function implementations rather than the off-chain fallback
//! that near-sdk's own unit tests can select.

use near_sdk::mock::MockAction;
use near_sdk::test_utils::{VMContextBuilder, get_created_receipts};
use near_sdk::universal_state_init::{
    PublicKeyHandle, RawStateInit, UniversalStateInit, UniversalStateInitV1,
};
use near_sdk::{GlobalContractId, NearToken, Promise, env, testing_env};

/// nearcore's `test_derive_universal_account_id` key-only vector.
fn key_only() -> RawStateInit {
    UniversalStateInitV1::default().with_access_key(PublicKeyHandle::MLDSA65Hash([0x11; 32])).into()
}
const KEY_ONLY_ID: &str = "0ux8te7g99f9kqzdtp9h4qnwt9aczpgayymmtbdc50w199rcw3at1g";

/// V1 with data keys `b`, `a` in that order: accepted on chain, but not what encoding the decoded
/// value gives back.
fn unsorted_data_keys() -> RawStateInit {
    let mut bytes = vec![0, 0];
    bytes.extend_from_slice(&2u32.to_le_bytes());
    for key in [b'b', b'a'] {
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.push(key);
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    RawStateInit(bytes)
}
const UNSORTED_ID: &str = "0u5v0d8z8y0fpzhtczvbw31a8tpxsxap2f9xkzygek2ht3t7pt34ag";
const UNSORTED_REENCODED_ID: &str = "0uxxf505cvpqd83cqj71wpya6mbmfh0tmjbqnjk6kwcze985r6f0tg";

#[test]
fn host_function_derives_the_id_of_the_exact_bytes() {
    testing_env!(VMContextBuilder::new().build());

    let gas_before = env::used_gas();
    assert_eq!(env::universal_state_init_to_account_id(key_only()).as_str(), KEY_ONLY_ID);
    // Charged gas shows the call went through the mocked host function.
    assert!(env::used_gas() > gas_before);

    let raw = unsorted_data_keys();
    assert_eq!(env::universal_state_init_to_account_id(&raw).as_str(), UNSORTED_ID);
    assert_eq!(raw.derive_account_id().as_str(), UNSORTED_ID);
    let reencoded = UniversalStateInit::from_raw(&raw).unwrap().to_raw();
    assert_ne!(reencoded, raw);
    assert_eq!(env::universal_state_init_to_account_id(reencoded).as_str(), UNSORTED_REENCODED_ID);
}

#[test]
fn promise_batch_action_carries_the_bytes_verbatim() {
    testing_env!(VMContextBuilder::new().build());

    let raw = unsorted_data_keys();
    let deposit = NearToken::from_millinear(10);
    let promise = env::promise_batch_create(&env::universal_state_init_to_account_id(&raw));
    env::promise_batch_action_universal_state_init(promise, raw.0.as_slice(), deposit);

    let receipts = get_created_receipts();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].receiver_id.as_str(), UNSORTED_ID);
    assert!(matches!(
        receipts[0].actions.as_slice(),
        [MockAction::UniversalStateInit { state_init, amount, .. }]
            if *state_init == raw.0 && *amount == deposit
    ));
}

#[test]
fn promise_targets_the_id_of_the_bytes_it_sends() {
    testing_env!(VMContextBuilder::new().build());

    let typed = UniversalStateInitV1::default()
        .with_code(GlobalContractId::AccountId("code.near".parse().unwrap()))
        .with_data_entry(b"owner", b"alice.near");
    let deposit = NearToken::from_millinear(10);
    let promise = |raw: RawStateInit| {
        Promise::new(env::universal_state_init_to_account_id(&raw))
            .universal_state_init(raw, deposit)
    };
    promise(unsorted_data_keys()).and(promise(typed.clone().into())).detach();

    let receipts = get_created_receipts();
    assert_eq!(receipts.len(), 2);
    for (receipt, raw) in receipts.iter().zip([unsorted_data_keys(), RawStateInit::from(typed)]) {
        assert_eq!(receipt.receiver_id, raw.derive_account_id());
        assert!(matches!(
            receipt.actions.as_slice(),
            [MockAction::UniversalStateInit { state_init, amount, .. }]
                if *state_init == raw.0 && *amount == deposit
        ));
    }
    assert_eq!(receipts[0].receiver_id.as_str(), UNSORTED_ID);
}
