use near_sdk::near;

#[near(serializers = [json(rename_all = "camelCase")])]
pub struct JsonOnly {
    pub first_field: u8,
}

#[near(serializers = [json(rename_all = "camelCase"), borsh])]
pub struct JsonThenBorsh {
    pub first_field: u8,
}

#[near(serializers = [borsh(use_discriminant = true), json(rename_all = "camelCase")])]
#[derive(Clone, Copy)]
pub enum BorshThenJson {
    FirstVariant = 5,
}

#[test]
fn json_args_are_applied() {
    let json = near_sdk::serde_json::to_string(&JsonOnly { first_field: 1 }).unwrap();
    assert_eq!(json, r#"{"firstField":1}"#);
}

#[test]
fn json_args_are_applied_when_borsh_follows() {
    let value = JsonThenBorsh { first_field: 1 };
    assert_eq!(near_sdk::serde_json::to_string(&value).unwrap(), r#"{"firstField":1}"#);
    assert_eq!(near_sdk::borsh::to_vec(&value).unwrap(), vec![1]);
}

#[test]
fn borsh_args_are_kept_when_json_args_follow() {
    let value = BorshThenJson::FirstVariant;
    assert_eq!(near_sdk::serde_json::to_string(&value).unwrap(), r#""firstVariant""#);
    assert_eq!(near_sdk::borsh::to_vec(&value).unwrap(), vec![5]);
}
