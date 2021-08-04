use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Helper class to serialize/deserialize `Vec<u8>` to base64 string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct Base64VecU8(#[serde(with = "base64_bytes")] pub Vec<u8>);

impl From<Vec<u8>> for Base64VecU8 {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<Base64VecU8> for Vec<u8> {
    fn from(v: Base64VecU8) -> Vec<u8> {
        v.0
    }
}

/// Convenience module to allow anotating a serde structure as base64 bytes.
///
/// # Example
/// ```ignore
/// use serde::{Serialize, Deserialize};
/// use near_sdk::json_types::base64_bytes;
///
/// #[derive(Serialize, Deserialize)]
/// struct NewStruct {
///     #[serde(with = "base64_bytes")]
///     field: Vec<u8>,
/// }
/// ```
mod base64_bytes {
    use super::*;
    use serde::de;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&base64::encode(&bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        base64::decode(s.as_str()).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_serde {
        ($v: expr) => {
            let a: Vec<u8> = $v;
            let wrapped_a: Base64VecU8 = a.clone().into();
            let b: Vec<u8> = wrapped_a.clone().into();
            assert_eq!(a, b);

            let str: String = serde_json::to_string(&wrapped_a).unwrap();
            let deser_a: Base64VecU8 = serde_json::from_str(&str).unwrap();
            assert_eq!(a, deser_a.0);
        };
    }

    #[test]
    fn test_empty() {
        test_serde!(vec![]);
    }

    #[test]
    fn test_basic() {
        test_serde!(vec![0]);
        test_serde!(vec![1]);
        test_serde!(vec![1, 2, 3]);
        test_serde!(b"abc".to_vec());
        test_serde!(vec![3, 255, 255, 13, 0, 23]);
    }

    #[test]
    fn test_long() {
        test_serde!(vec![123; 16000]);
    }

    #[test]
    fn test_manual() {
        let a = vec![100, 121, 31, 20, 0, 23, 32];
        let a_str = serde_json::to_string(&Base64VecU8(a.clone())).unwrap();
        assert_eq!(a_str, String::from("\"ZHkfFAAXIA==\""));
        let a_deser: Base64VecU8 = serde_json::from_str(&a_str).unwrap();
        assert_eq!(a_deser.0, a);
    }
}
