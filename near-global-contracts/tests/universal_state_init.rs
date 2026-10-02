//! Public universal state-init encoding, decoding, wire-format and identity contracts.

// Link the mock host functions when testing the on-chain hashing backend natively.
#[cfg(all(near, feature = "__near-sdk-unit-testing"))]
use near_sdk as _;

#[cfg(any(feature = "borsh", feature = "near-primitives-interop", feature = "schemars-v0_8"))]
use near_global_contracts::RawStateInit;
#[cfg(any(feature = "borsh", feature = "serde"))]
use near_global_contracts::{
    GlobalContractId, PublicKeyHandle, UniversalStateInit, UniversalStateInitV1,
};

#[cfg(feature = "borsh")]
fn key_only() -> UniversalStateInit {
    UniversalStateInit::V1(
        UniversalStateInitV1::default().with_access_key(PublicKeyHandle::MLDSA65Hash([0x11; 32])),
    )
}

#[test]
#[cfg(feature = "borsh")]
fn try_from_raw_rejects_trailing_and_truncated_bytes() {
    let bytes = key_only().to_raw().0;
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(UniversalStateInit::try_from(RawStateInit(trailing)).is_err());
    assert!(UniversalStateInit::try_from(RawStateInit(bytes[..bytes.len() - 1].to_vec())).is_err());
    // An unknown version tag is not a state init this crate can type.
    assert!(UniversalStateInit::try_from(RawStateInit(vec![1])).is_err());
}

/// State inits with their exact nearcore 2.14 encoding and id, checked byte for byte against
/// `near-primitives =0.38.0-rc.2` (its `UniversalStateInit::to_raw` and 0u derivation)
/// before being pinned here. The last one is the NEP-655 Appendix A wallet vector.
#[cfg(feature = "borsh")]
struct KnownAnswer {
    name: &'static str,
    state_init: UniversalStateInitV1,
    bytes: &'static str,
    id: &'static str,
}

#[cfg(feature = "borsh")]
fn known_answers() -> Vec<KnownAnswer> {
    let v1 = UniversalStateInitV1::default;
    let wallet_code = || {
        GlobalContractId::AccountId("0sb0d7ef4f935c6ef78e08ad03569767aaec4223a3".parse().unwrap())
    };
    vec![
        KnownAnswer {
            name: "empty",
            state_init: v1(),
            bytes: "00000000000000000000",
            id: "0u1kajgpx8a97y8ap8y03pvt8kbm2p2cn9k5h17bgw1wa21j88865g",
        },
        KnownAnswer {
            name: "code by account id",
            state_init: v1().with_code(GlobalContractId::AccountId("code.near".parse().unwrap())),
            bytes: "00010109000000636f64652e6e6561720000000000000000",
            id: "0uartat3agnh029e3nqsqzxtz5pd3130stv6a2eafszq4vgg56kaag",
        },
        KnownAnswer {
            name: "code by hash",
            state_init: v1().with_code(GlobalContractId::CodeHash([0x22; 32])),
            bytes: "00010022222222222222222222222222222222222222222222222222222222222222220000000000000000",
            id: "0uk6fe1b5tnfxn87yhtc36czcehzd4ngf25crhj17871kax56nhxx0",
        },
        KnownAnswer {
            name: "data inserted out of order, empty key, prefix keys",
            state_init: v1().with_data_entry(b"b", b"2")
                .with_data_entry(b"a", b"1")
                .with_data_entry(b"ab", b"")
                .with_data_entry(b"", b"empty-key")
                .with_data_entry([0xff, 0x00], [0x00; 3]),
            bytes: "0000050000000000000009000000656d7074792d6b657901000000610100000031020000006162000000000100000062010000003202000000ff000300000000000000000000",
            id: "0u88nzp537q37vgx90hrjbmqv7dhbc776w4ynj514djahna1cnce5g",
        },
        KnownAnswer {
            name: "nearcore key-only",
            state_init: v1().with_access_key(PublicKeyHandle::MLDSA65Hash([0x11; 32])),
            bytes: "00000000000001000000031111111111111111111111111111111111111111111111111111111111111111",
            id: "0ux8te7g99f9kqzdtp9h4qnwt9aczpgayymmtbdc50w199rcw3at1g",
        },
        KnownAnswer {
            name: "nearcore contract",
            state_init: v1().with_code(GlobalContractId::CodeHash([0x22; 32])).with_data_entry(b"key", b"value"),
            bytes: "000100222222222222222222222222222222222222222222222222222222222222222201000000030000006b65790500000076616c756500000000",
            id: "0uzvdgbyea2rd8ywx0kw3cg4vc0ez1x5fc2gyks4fdz9ae0xxvzan0",
        },
        KnownAnswer {
            name: "NEP-655 Appendix A wallet",
            state_init: v1().with_code(wallet_code()).with_data_entry(
                b"",
                hex::decode("01000000008565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466100e00000000000000000000000000000000000000000000").unwrap(),
            ),
            bytes: "0001012a00000030736230643765663466393335633665663738653038616430333536393736376161656334323233613301000000000000003d00000001000000008565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466100e0000000000000000000000000000000000000000000000000000",
            id: "0u4bfkw2qvgfzbf7zzkxykcppqymn0p2hbayjee3ygzrbhmmtyejx0",
        },
    ]
}

#[test]
#[cfg(feature = "borsh")]
fn matches_nearcore_and_nep655_known_answers() {
    for KnownAnswer { name, state_init, bytes, id } in known_answers() {
        let state_init = UniversalStateInit::from(state_init);
        let raw = RawStateInit(hex::decode(bytes).unwrap());
        assert_eq!(state_init.to_raw(), raw, "{name}: bytes");
        assert_eq!(UniversalStateInit::try_from(&raw).unwrap(), state_init, "{name}: decode");
        assert_eq!(raw.derive_account_id().as_str(), id, "{name}: id");
        assert_eq!(state_init.derive_account_id().as_str(), id, "{name}: typed id");
    }
}

/// Ids of larger state inits (every key kind, everything together), from the same
/// differential run against nearcore; the id pins the bytes.
#[test]
#[cfg(feature = "borsh")]
fn matches_nearcore_known_answer_ids() {
    let mut ed25519_high = [0; 32];
    ed25519_high[0] = 0xff;
    let keys = [
        PublicKeyHandle::MLDSA65Hash([0x00; 32]),
        PublicKeyHandle::SECP256K1([0x01; 64]),
        PublicKeyHandle::ED25519(ed25519_high),
        PublicKeyHandle::ED25519([0x00; 32]),
        PublicKeyHandle::MLDSA65Hash([0xff; 32]),
        PublicKeyHandle::SECP256K1([0x00; 64]),
    ];
    let keys_only =
        keys.iter().cloned().fold(UniversalStateInitV1::default(), |v1, k| v1.with_access_key(k));
    let everything = keys_only
        .clone()
        .with_code(GlobalContractId::AccountId(
            "0sb0d7ef4f935c6ef78e08ad03569767aaec4223a3".parse().unwrap(),
        ))
        .with_data_entry(b"k", b"v");
    for (state_init, len, id) in [
        (keys_only, 272, "0upcrgsbq81vedn1tsvw10heb4rjzdjmasxc8wayh26jpj5nemrqsg"),
        (everything, 329, "0utr5yk5pdhjg932r5px1mf7vd6evsh0x46fn6g3zscys5b2y6d480"),
    ] {
        let raw = RawStateInit::from(state_init);
        assert_eq!(raw.0.len(), len);
        assert_eq!(raw.derive_account_id().as_str(), id);
    }
}

/// Non-canonical bytes are accepted and derive their own id, which re-encoding loses. Values
/// match nearcore's 0u derivation and its `UniversalStateInit::from_raw`.
#[test]
#[cfg(feature = "borsh")]
fn non_canonical_bytes_keep_their_own_id() {
    fn entries(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
        let mut bytes = vec![0, 0];
        bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, value) in entries {
            bytes.extend_from_slice(&(key.len() as u32).to_le_bytes());
            bytes.extend_from_slice(key);
            bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
            bytes.extend_from_slice(value);
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes
    }

    let unsorted = RawStateInit(entries(&[(b"b", b""), (b"a", b"")]));
    let decoded = UniversalStateInit::try_from(&unsorted).unwrap();
    assert_ne!(decoded.to_raw(), unsorted);
    assert_eq!(
        unsorted.derive_account_id().as_str(),
        "0u5v0d8z8y0fpzhtczvbw31a8tpxsxap2f9xkzygek2ht3t7pt34ag"
    );
    assert_eq!(
        decoded.derive_account_id().as_str(),
        "0uxxf505cvpqd83cqj71wpya6mbmfh0tmjbqnjk6kwcze985r6f0tg"
    );

    // Duplicate keys: last one wins on decode, like nearcore.
    let duplicate = RawStateInit(entries(&[(b"a", b"1"), (b"a", b"2")]));
    let decoded = UniversalStateInit::try_from(&duplicate).unwrap();
    assert_eq!(decoded.data().get(&b"a"[..]), Some(&b"2".to_vec()));
    assert_ne!(decoded.to_raw(), duplicate);
    assert_ne!(decoded.derive_account_id(), duplicate.derive_account_id());

    // Tag 2 (a full ML-DSA-65 key) is not a valid handle.
    let mut full_key = vec![0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2];
    full_key.extend_from_slice(&[0; 32]);
    assert!(UniversalStateInit::try_from(RawStateInit(full_key)).is_err());
}

#[test]
#[cfg(feature = "near-primitives-interop")]
fn raw_state_init_converts_to_and_from_nearcore() {
    use near_primitives_core::universal_state_init::RawStateInit as NearcoreRawStateInit;

    let raw = RawStateInit(vec![0, 1, 2]);
    let nearcore = NearcoreRawStateInit::from(raw.clone());
    assert_eq!(nearcore.0, raw.0);
    assert_eq!(RawStateInit::from(nearcore), raw);
}

#[test]
#[cfg(feature = "borsh")]
fn access_keys_are_ordered_by_tag_then_bytes() {
    let state_init = UniversalStateInitV1::default()
        .with_access_key(PublicKeyHandle::MLDSA65Hash([0x00; 32]))
        .with_access_key(PublicKeyHandle::SECP256K1([0x00; 64]))
        .with_access_key(PublicKeyHandle::ED25519([0xff; 32]))
        .with_access_key(PublicKeyHandle::ED25519([0x00; 32]));
    let tags: Vec<u8> =
        state_init.access_keys.iter().map(|k| borsh::to_vec(k).unwrap()[0]).collect();
    assert_eq!(tags, [0, 0, 1, 3]);
    let first_two: Vec<&[u8]> =
        state_init.access_keys.iter().take(2).map(|k| k.key_data()).collect();
    assert_eq!(first_two, [&[0x00; 32][..], &[0xff; 32][..]]);
}

/// Like nearcore's `RawStateInit`: a length-prefixed byte vector in borsh, a base64 string
/// in JSON, and the id hashes the bytes without either wrapper.
#[test]
#[cfg(all(feature = "serde", feature = "borsh"))]
fn raw_state_init_wire_forms() {
    let raw = key_only().to_raw();
    let mut expected = (raw.0.len() as u32).to_le_bytes().to_vec();
    expected.extend_from_slice(&raw.0);
    assert_eq!(borsh::to_vec(&raw).unwrap(), expected);
    assert_eq!(borsh::from_slice::<RawStateInit>(&expected).unwrap(), raw);

    let json = serde_json::to_value(RawStateInit(b"key".to_vec())).unwrap();
    assert_eq!(json, serde_json::json!("a2V5"));
    assert_eq!(
        serde_json::from_value::<RawStateInit>(json).unwrap(),
        RawStateInit(b"key".to_vec())
    );

    assert_eq!(raw.derive_account_id(), key_only().derive_account_id());
    assert_ne!(raw.derive_account_id(), RawStateInit(expected).derive_account_id());
}

#[test]
#[cfg(feature = "schemars-v0_8")]
fn raw_state_init_json_schema_is_base64_string() {
    let schema = serde_json::to_value(schemars_v0_8::schema_for!(RawStateInit)).unwrap();
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["contentEncoding"], "base64");
}

#[cfg(feature = "serde")]
fn json_fixture() -> UniversalStateInitV1 {
    UniversalStateInitV1::default()
        .with_code(GlobalContractId::AccountId("code.near".parse().unwrap()))
        .with_data_entry([0xff, 0x00, 0x80], [0x00, 0xfe, 0x81])
        .with_data_entry(b"", b"\0")
        .with_access_key(PublicKeyHandle::ED25519([0x11; 32]))
        .with_access_key(PublicKeyHandle::SECP256K1([0x22; 64]))
        .with_access_key(PublicKeyHandle::MLDSA65Hash([0x33; 32]))
}

#[test]
#[cfg(feature = "serde")]
fn typed_json_round_trip_preserves_binary_data_and_access_keys() {
    let v1 = json_fixture();
    let json = serde_json::to_value(&v1).unwrap();
    assert_eq!(json["code"], serde_json::json!({"account_id": "code.near"}));
    assert_eq!(json["data"], serde_json::json!({"/wCA": "AP6B", "": "AA=="}));
    let keys = json["access_keys"].as_array().unwrap();
    assert_eq!(keys.len(), 3);
    for (key, prefix) in keys.iter().zip(["ed25519:", "secp256k1:", "ml-dsa-65-hash:"]) {
        assert!(key.as_str().unwrap().starts_with(prefix));
    }
    assert_eq!(serde_json::from_value::<UniversalStateInitV1>(json.clone()).unwrap(), v1);

    let typed = UniversalStateInit::from(v1);
    let tagged = serde_json::json!({"v1": json});
    assert_eq!(serde_json::to_value(&typed).unwrap(), tagged);
    assert_eq!(serde_json::from_value::<UniversalStateInit>(tagged).unwrap(), typed);

    let by_hash =
        UniversalStateInit::from(json_fixture().with_code(GlobalContractId::CodeHash([0x44; 32])));
    let json = serde_json::to_value(&by_hash).unwrap();
    assert!(json["v1"]["code"]["hash"].is_string());
    assert_eq!(serde_json::from_value::<UniversalStateInit>(json).unwrap(), by_hash);
}

#[test]
#[cfg(feature = "serde")]
fn typed_json_defaults_and_omits_empty_fields() {
    let empty = UniversalStateInitV1::default();
    assert_eq!(serde_json::to_value(&empty).unwrap(), serde_json::json!({}));
    for json in
        [serde_json::json!({}), serde_json::json!({"code": null, "data": {}, "access_keys": []})]
    {
        assert_eq!(serde_json::from_value::<UniversalStateInitV1>(json.clone()).unwrap(), empty);
        let tagged = serde_json::json!({"v1": json});
        assert_eq!(
            serde_json::from_value::<UniversalStateInit>(tagged).unwrap(),
            UniversalStateInit::from(empty.clone())
        );
    }
    assert_eq!(
        serde_json::to_value(UniversalStateInit::from(empty)).unwrap(),
        serde_json::json!({"v1": {}})
    );
}

#[test]
#[cfg(feature = "schemars-v0_8")]
fn typed_json_schemas_describe_tagged_binary_data_and_access_keys() {
    let v1_schema = serde_json::to_value(schemars_v0_8::schema_for!(UniversalStateInitV1)).unwrap();
    assert_eq!(v1_schema["type"], "object");
    assert!(v1_schema.get("required").is_none());
    let properties = &v1_schema["properties"];
    assert_eq!(properties["data"]["type"], "object");
    assert_eq!(properties["data"]["additionalProperties"]["type"], "string");
    assert_eq!(properties["data"]["additionalProperties"]["contentEncoding"], "base64");
    assert_eq!(properties["access_keys"]["type"], "array");
    assert_eq!(properties["access_keys"]["items"]["type"], "string");
    assert_eq!(properties["access_keys"]["uniqueItems"], true);
    assert!(properties.get("code").is_some());

    let schema = serde_json::to_value(schemars_v0_8::schema_for!(UniversalStateInit)).unwrap();
    let variant = &schema["oneOf"][0];
    assert_eq!(variant["required"], serde_json::json!(["v1"]));
    assert_eq!(variant["properties"]["v1"]["$ref"], "#/definitions/UniversalStateInitV1");
    assert_eq!(schema["definitions"]["UniversalStateInitV1"]["properties"], *properties);

    // Check the nonempty JSON shape as well: empty fixtures would hide a byte-array schema.
    let json = serde_json::to_value(UniversalStateInit::from(json_fixture())).unwrap();
    let payload = &json["v1"];
    assert!(payload["data"].as_object().unwrap().values().all(serde_json::Value::is_string));
    assert!(payload["access_keys"].as_array().unwrap().iter().all(serde_json::Value::is_string));
}

#[test]
#[cfg(all(feature = "serde", feature = "borsh"))]
fn typed_json_inspection_does_not_establish_raw_canonicity() {
    let raw = RawStateInit(
        hex::decode("00000200000001000000620000000001000000610000000000000000").unwrap(),
    );
    let typed = UniversalStateInit::try_from(&raw).unwrap();
    let json = serde_json::to_value(&typed).unwrap();
    let inspected: UniversalStateInit = serde_json::from_value(json).unwrap();
    assert_eq!(inspected, typed);
    assert_ne!(inspected.to_raw(), raw);
    assert_ne!(inspected.derive_account_id(), raw.derive_account_id());
    let wire = serde_json::to_value(&raw).unwrap();
    assert_eq!(serde_json::from_value::<RawStateInit>(wire).unwrap(), raw);
}
