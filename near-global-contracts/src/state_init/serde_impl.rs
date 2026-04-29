use crate::GlobalContractId;
use serde_with::base64::Base64;
use std::collections::BTreeMap;

#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
struct StateInitV1Helper {
    code: GlobalContractId,
    #[serde_as(as = "BTreeMap<Base64, Base64>")]
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl serde::Serialize for super::StateInitV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        StateInitV1Helper { code: self.code.clone(), data: self.data.clone() }.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for super::StateInitV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = StateInitV1Helper::deserialize(deserializer)?;
        Ok(Self { code: helper.code, data: helper.data })
    }
}

#[cfg(test)]
mod tests {
    use near_sdk_core::json_types::Base58CryptoHash;

    use crate::{GlobalContractId, StateInit, StateInitV1};

    #[test]
    fn test_state_init_json_serialization_externally_tagged() {
        let hash: Base58CryptoHash =
            "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ".parse().unwrap();
        let state_init = StateInit::V1(StateInitV1::code(GlobalContractId::CodeHash(hash)));

        let json = serde_json::to_string(&state_init).unwrap();
        // Must use serde's default externally tagged format to match nearcore
        assert!(json.starts_with(r#"{"V1":"#), "expected externally tagged format, got: {json}");

        let deserialized: StateInit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state_init);

        // Old internally tagged format must NOT deserialize
        let old_format = r#"{"version":"v1","code":{"hash":"4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"},"data":{}}"#;
        assert!(
            serde_json::from_str::<StateInit>(old_format).is_err(),
            "old internally tagged format should be rejected"
        );
    }
}
