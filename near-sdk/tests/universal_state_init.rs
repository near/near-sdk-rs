//! Universal state init (NEP-655) through the mocked host functions.
//!
//! An integration test links near-sdk without `cfg(test)`, so with `unit-testing` every `env` call
//! here goes through nearcore's host function implementations rather than the off-chain fallback
//! that near-sdk's own unit tests can select. The mock answers the host's canonicality question
//! with nearcore's own strict decoder, as the chain does from protocol version 88.

use near_sdk::mock::MockAction;
use near_sdk::test_utils::{VMContextBuilder, get_created_receipts};
use near_sdk::universal_state_init::{
    PublicKeyHandle, RawStateInit, StateInitError, UniversalStateInit, UniversalStateInitV1,
};
use near_sdk::{GlobalContractId, NearToken, Promise, env, testing_env};

/// nearcore's key-only universal account id vector.
fn key_only() -> RawStateInit {
    UniversalStateInitV1::default().with_access_key(PublicKeyHandle::MLDSA65Hash([0x11; 32])).into()
}
const KEY_ONLY_ID: &str = "0ux8te7g99f9kqzdtp9h4qnwt9aczpgayymmtbdc50w199rcw3at1g";

/// Non-canonical V1 bytes with data keys `b`, `a`.
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
/// The id the legacy host (protocol version 87) hashed `unsorted_data_keys` to.
const UNSORTED_LEGACY_ID: &str = "0u5v0d8z8y0fpzhtczvbw31a8tpxsxap2f9xkzygek2ht3t7pt34ag";
const UNSORTED_REENCODED_ID: &str = "0uxxf505cvpqd83cqj71wpya6mbmfh0tmjbqnjk6kwcze985r6f0tg";

#[test]
fn host_derives_the_id_of_canonical_bytes() {
    testing_env!(VMContextBuilder::new().build());

    let gas_before = env::used_gas();
    assert_eq!(env::universal_state_init_to_account_id(&key_only()).unwrap().as_str(), KEY_ONLY_ID);
    // Charged gas shows the call went through the mocked host function.
    assert!(env::used_gas() > gas_before);
}

#[test]
fn host_rejects_noncanonical_and_malformed_bytes() {
    testing_env!(VMContextBuilder::new().build());

    let raw = unsorted_data_keys();
    let gas_before = env::used_gas();
    assert_eq!(env::universal_state_init_to_account_id(&raw), Err(StateInitError::NonCanonical));
    // The per-byte fee pays for the check, so it is charged on rejection too.
    assert!(env::used_gas() > gas_before);

    // Re-encoding gives canonical bytes, and so a different account.
    let reencoded = UniversalStateInit::try_from(&raw).unwrap().to_raw();
    assert_ne!(reencoded, raw);
    assert_eq!(
        env::universal_state_init_to_account_id(&reencoded).unwrap().as_str(),
        UNSORTED_REENCODED_ID,
    );

    let unknown_version = RawStateInit(vec![0xff, 0x00, 0x80, 0x01]);
    assert_eq!(
        env::universal_state_init_to_account_id(&unknown_version),
        Err(StateInitError::Malformed),
    );
}

/// The host (nearcore's strict decoder) and `RawStateInit::try_derive_account_id` (the SDK's local
/// check) agree on every canonicality vector, down to which error they report.
#[test]
fn host_and_local_validation_agree() {
    testing_env!(VMContextBuilder::new().build());

    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../near-global-contracts/tests/data/universal_state_init.json"
    ))
    .unwrap();
    for (expect_valid, vectors) in [(true, &vectors["valid"]), (false, &vectors["invalid"])] {
        for vector in vectors.as_array().unwrap() {
            let name = vector["name"].as_str().unwrap();
            let raw = RawStateInit(hex::decode(vector["hex"].as_str().unwrap()).unwrap());
            let host = env::universal_state_init_to_account_id(&raw);
            assert_eq!(host, raw.try_derive_account_id(), "{name}");
            assert_eq!(host.is_ok(), expect_valid, "{name}");
            if let Some(id) = vector.get("account_id") {
                assert_eq!(host.unwrap().as_str(), id.as_str().unwrap(), "{name}");
            }
        }
    }
}

/// The action host function does not validate: the bytes go into the action verbatim, and the
/// runtime rejects a non-canonical state init when it validates the new receipt.
#[test]
fn promise_batch_action_carries_the_bytes_verbatim() {
    testing_env!(VMContextBuilder::new().build());

    let raw = unsorted_data_keys();
    let deposit = NearToken::from_millinear(10);
    let promise = env::promise_batch_create(&UNSORTED_LEGACY_ID.parse().unwrap());
    env::promise_batch_action_universal_state_init(promise, &raw, deposit);

    let receipts = get_created_receipts();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].receiver_id.as_str(), UNSORTED_LEGACY_ID);
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
        Promise::new(env::universal_state_init_to_account_id(&raw).unwrap())
            .universal_state_init(raw, deposit)
    };
    promise(key_only()).and(promise(typed.clone().into())).detach();

    let receipts = get_created_receipts();
    assert_eq!(receipts.len(), 2);
    for (receipt, raw) in receipts.iter().zip([key_only(), RawStateInit::from(typed)]) {
        assert_eq!(receipt.receiver_id, raw.try_derive_account_id().unwrap());
        assert!(matches!(
            receipt.actions.as_slice(),
            [MockAction::UniversalStateInit { state_init, amount, .. }]
                if *state_init == raw.0 && *amount == deposit
        ));
    }
    assert_eq!(receipts[0].receiver_id.as_str(), KEY_ONLY_ID);
}
