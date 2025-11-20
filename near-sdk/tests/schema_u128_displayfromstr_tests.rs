// Test for Vec<u128> field with serde_as using near-sdk's U128 type
// Demonstrates that u128::MAX values in a vector can be properly serialized as strings,
// since JSON doesn't support integers larger than 2^53-1

use near_sdk::{json_types::U128, near, serde_with::FromInto};

#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq)]
pub struct AmountWrapper {
    #[serde_as(as = "Vec<FromInto<U128>>")]
    pub amounts: Vec<u128>,
}

#[test]
fn test_vec_u128_with_serde_as_u128() {
    // u128::MAX = 340282366920938463463374607431768211455
    // This value cannot be represented in JSON as a number (exceeds 2^53-1)
    // Using serde_as with Vec<FromInto<U128>> serializes each element as a string

    let wrapper = AmountWrapper { amounts: vec![0, 12345, u128::MAX / 2, u128::MAX] };

    // Serialize to JSON - each u128 should be a string, not a number
    let json = serde_json::to_string(&wrapper).unwrap();
    println!("Serialized Vec<u128>: {}", json);

    let expected = r#"{"amounts":["0","12345","170141183460469231731687303715884105727","340282366920938463463374607431768211455"]}"#;
    assert_eq!(
        json, expected,
        "Vec<u128> elements must be serialized as strings to avoid precision loss"
    );

    // Deserialize back - should work correctly
    let deserialized: AmountWrapper = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.amounts[0], 0, "First element should be 0");
    assert_eq!(deserialized.amounts[1], 12345, "Second element should be 12345");
    assert_eq!(deserialized.amounts[2], u128::MAX / 2, "Third element should be u128::MAX / 2");
    assert_eq!(deserialized.amounts[3], u128::MAX, "Fourth element should be u128::MAX");

    // Verify the value is correct
    assert_eq!(wrapper, deserialized, "Round-trip serialization must preserve all values");
}

#[cfg(feature = "abi")]
#[test]
fn test_vec_u128_schema_generation() {
    use near_sdk::schemars::schema_for;

    // Generate schema for the struct with Vec<u128>
    let schema = schema_for!(AmountWrapper);
    let schema_json = serde_json::to_string_pretty(&schema).unwrap();

    println!("Generated schema:\n{}", schema_json);

    // Convert schema to JSON to check the type
    let schema_value: serde_json::Value = serde_json::from_str(&schema_json).unwrap();

    // Check that Vec<u128> elements are typed as string (not integer)
    let items_type = &schema_value["properties"]["amounts"]["items"]["type"];

    assert_eq!(
        items_type.as_str().unwrap(),
        "string",
        "Vec<u128> elements must have type 'string' in schema (not 'integer') to match serialization behavior"
    );
}
