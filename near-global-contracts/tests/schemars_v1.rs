//! `schemars` 1.x schemas of the serde types, and their agreement with the 0.8 ones.
#![cfg(feature = "schemars-v1")]

// Link the mock host functions when testing the on-chain hashing backend natively.
#[cfg(all(near, feature = "__near-sdk-unit-testing"))]
use near_sdk as _;

use near_global_contracts::{
    GlobalContractId, PublicKeyHandle, RawStateInit, StateInit, StateInitV1, UniversalStateInit,
    UniversalStateInitV1,
};
use serde_json::{Value, json};

fn schema_v1<T: schemars_v1::JsonSchema>() -> Value {
    serde_json::to_value(schemars_v1::schema_for!(T)).unwrap()
}

/// Follows a local `$ref` such as `#/$defs/GlobalContractId` within `root`.
fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    match schema["$ref"].as_str() {
        Some(pointer) => root.pointer(pointer.strip_prefix('#').unwrap()).unwrap(),
        None => schema,
    }
}

#[test]
fn global_contract_id() {
    let schema = schema_v1::<GlobalContractId>();
    assert_eq!(schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert_eq!(schema["title"], "GlobalContractId");
    let [hash, account_id] = schema["oneOf"].as_array().unwrap().as_slice() else {
        panic!("expected two variants: {schema:#}")
    };
    assert_eq!(hash["required"], json!(["hash"]));
    assert_eq!(hash["additionalProperties"], false);
    // Base58, not a byte array.
    assert_eq!(hash["properties"]["hash"], json!({"type": "string"}));
    assert_eq!(account_id["required"], json!(["account_id"]));
    let account_id = resolve(&schema, &account_id["properties"]["account_id"]);
    assert_eq!(account_id["type"], "string");
}

#[test]
fn state_init() {
    let schema = schema_v1::<StateInit>();
    let [v1] = schema["oneOf"].as_array().unwrap().as_slice() else {
        panic!("expected one variant: {schema:#}")
    };
    // The tag keeps the Rust variant name: no `rename_all`, unlike `UniversalStateInit`.
    assert_eq!(v1["required"], json!(["V1"]));
    assert_eq!(v1["properties"]["V1"]["$ref"], "#/$defs/StateInitV1");
    assert_eq!(schema["$defs"]["StateInitV1"], strip_root(schema_v1::<StateInitV1>()));
}

#[test]
fn state_init_v1() {
    let schema = schema_v1::<StateInitV1>();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["code", "data"]));
    assert_eq!(schema["properties"]["code"]["$ref"], "#/$defs/GlobalContractId");
    assert_eq!(schema["$defs"]["GlobalContractId"], strip_root(schema_v1::<GlobalContractId>()));
    assert_base64_map(&schema["properties"]["data"]);
}

#[test]
fn raw_state_init() {
    let schema = schema_v1::<RawStateInit>();
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["contentEncoding"], "base64");
}

#[test]
fn universal_state_init() {
    let schema = schema_v1::<UniversalStateInit>();
    let [v1] = schema["oneOf"].as_array().unwrap().as_slice() else {
        panic!("expected one variant: {schema:#}")
    };
    assert_eq!(v1["required"], json!(["v1"]));
    assert_eq!(v1["additionalProperties"], false);
    assert_eq!(v1["properties"]["v1"]["$ref"], "#/$defs/UniversalStateInitV1");
    assert_eq!(
        schema["$defs"]["UniversalStateInitV1"],
        strip_root(schema_v1::<UniversalStateInitV1>())
    );
}

#[test]
fn universal_state_init_v1() {
    let schema = schema_v1::<UniversalStateInitV1>();
    assert_eq!(schema["type"], "object");
    // Every field has `serde(default)`.
    assert!(schema.get("required").is_none());
    let properties = &schema["properties"];
    assert_eq!(
        properties["code"]["anyOf"],
        json!([{"$ref": "#/$defs/GlobalContractId"}, {"type": "null"}])
    );
    assert_base64_map(&properties["data"]);
    assert_eq!(properties["access_keys"]["type"], "array");
    assert_eq!(properties["access_keys"]["uniqueItems"], true);
    // `PublicKeyHandle` is inlined as a string, as with 0.8.
    assert_eq!(properties["access_keys"]["items"], json!({"type": "string"}));
    assert_eq!(schema_v1::<PublicKeyHandle>()["type"], "string");
    assert!(schema["$defs"].get("PublicKeyHandle").is_none());
}

/// The schemas describe what serde actually writes for a value with every field populated.
#[test]
fn schemas_cover_serialized_values() {
    let v1 = UniversalStateInitV1::default()
        .with_code(GlobalContractId::CodeHash([0x22; 32]))
        .with_data_entry([0xff, 0x00], [0x00, 0xfe])
        .with_access_key(PublicKeyHandle::ED25519([0x11; 32]))
        .with_access_key(PublicKeyHandle::MLDSA65Hash([0x33; 32]));
    let json = serde_json::to_value(UniversalStateInit::from(v1)).unwrap();

    let schema = schema_v1::<UniversalStateInit>();
    let v1_schema = resolve(&schema, &schema["oneOf"][0]["properties"]["v1"]);
    let payload = &json["v1"];
    let fields = |object: &Value| -> Vec<String> {
        let mut keys: Vec<_> = object.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(fields(payload), fields(&v1_schema["properties"]));
    assert!(payload["code"]["hash"].is_string());
    assert!(payload["data"].as_object().unwrap().values().all(Value::is_string));
    assert!(payload["access_keys"].as_array().unwrap().iter().all(Value::is_string));

    let state_init = StateInit::from(
        StateInitV1::code(GlobalContractId::AccountId("code.near".parse().unwrap()))
            .with_data_entry(b"k", b"v"),
    );
    let json = serde_json::to_value(&state_init).unwrap();
    let schema = schema_v1::<StateInit>();
    let v1_schema = resolve(&schema, &schema["oneOf"][0]["properties"]["V1"]);
    assert_eq!(json!(fields(&json["V1"])), v1_schema["required"]);
}

/// With both features on, the v1 schemas match the 0.8 derive on the real types once the
/// differences between the schemars versions are normalized away. This keeps the private v1
/// mirrors in `src/schemars_v1.rs` in step with the types they stand in for, docs included.
#[test]
#[cfg(feature = "schemars-v0_8")]
fn matches_schemars_v0_8() {
    fn check<T: schemars_v0_8::JsonSchema + schemars_v1::JsonSchema>() {
        let v0_8 = serde_json::to_value(schemars_v0_8::schema_for!(T)).unwrap();
        let v1 = schema_v1::<T>();
        assert_eq!(
            normalize(&v0_8, &v0_8),
            normalize(&v1, &v1),
            "{}: the schemars-v1 mirror has drifted from the type\nv0_8: {v0_8:#}\nv1: {v1:#}",
            <T as schemars_v1::JsonSchema>::schema_name(),
        );
    }

    check::<GlobalContractId>();
    check::<StateInit>();
    check::<StateInitV1>();
    check::<RawStateInit>();
    check::<UniversalStateInit>();
    check::<UniversalStateInitV1>();
    check::<PublicKeyHandle>();
}

/// Inlines `$ref`s (0.8 `#/definitions/`, 1.x `#/$defs/`) and single-entry `allOf` wrappers
/// (0.8 puts a `$ref` next to a description that way), drops `$schema`, `title` and the
/// definitions themselves, and collapses whitespace in descriptions (1.x keeps doc comment
/// line breaks, 0.8 joins lines within a paragraph).
#[cfg(feature = "schemars-v0_8")]
fn normalize(root: &Value, schema: &Value) -> Value {
    match schema {
        Value::Object(object) => {
            let mut out = serde_json::Map::new();
            for (key, value) in object {
                match key.as_str() {
                    "$schema" | "title" | "definitions" | "$defs" => {}
                    "$ref" => {
                        let Value::Object(target) = normalize(root, resolve(root, schema)) else {
                            unreachable!()
                        };
                        out.extend(target);
                    }
                    "allOf" if value.as_array().is_some_and(|all| all.len() == 1) => {
                        let Value::Object(target) = normalize(root, &value[0]) else {
                            unreachable!()
                        };
                        out.extend(target);
                    }
                    "description" => {
                        let words: Vec<_> = value.as_str().unwrap().split_whitespace().collect();
                        out.insert(key.clone(), words.join(" ").into());
                    }
                    _ => {
                        out.insert(key.clone(), normalize(root, value));
                    }
                }
            }
            // A description next to a `$ref` belongs to the field, not the target.
            if let Some(description) = object.get("description") {
                let words: Vec<_> = description.as_str().unwrap().split_whitespace().collect();
                out.insert("description".into(), words.join(" ").into());
            }
            Value::Object(out)
        }
        Value::Array(items) => items.iter().map(|item| normalize(root, item)).collect(),
        other => other.clone(),
    }
}

/// A root schema as it appears under `$defs`.
fn strip_root(mut schema: Value) -> Value {
    let object = schema.as_object_mut().unwrap();
    for key in ["$schema", "title", "$defs"] {
        object.remove(key);
    }
    schema
}

fn assert_base64_map(schema: &Value) {
    assert_eq!(schema["type"], "object");
    assert_eq!(
        schema["additionalProperties"],
        json!({"type": "string", "contentEncoding": "base64"})
    );
}
