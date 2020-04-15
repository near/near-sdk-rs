//! Helper classes to serialize and deserialize large integer types into base-10 string
//! representations.
//! NOTE: JSON standard can only work with integer up to 53 bits. So we need helper classes for
//! 64-bit and 128-bit integers.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! impl_str_type {
    ($iden: ident, $ty: tt) => {
        #[derive(Debug, Clone, Copy, PartialEq, BorshDeserialize, BorshSerialize)]
        pub struct $iden(pub $ty);

        impl From<$ty> for $iden {
            fn from(v: $ty) -> Self {
                Self(v)
            }
        }

        impl From<$iden> for $ty {
            fn from(v: $iden) -> $ty {
                v.0
            }
        }

        impl Serialize for $iden {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $iden {
            fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
            where
                D: Deserializer<'de>,
            {
                let s: String = Deserialize::deserialize(deserializer)?;
                Ok(Self(
                    $ty::from_str_radix(&s, 10)
                        .map_err(|err| serde::de::Error::custom(err.to_string()))?,
                ))
            }
        }
    };
}

impl_str_type!(U128, u128);
impl_str_type!(U64, u64);
impl_str_type!(I128, i128);
impl_str_type!(I64, i64);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_serde {
        ($str_type: tt, $int_type: tt, $number: expr) => {
            let a: $int_type = $number;
            let str_a: $str_type = a.into();
            let b: $int_type = str_a.into();
            assert_eq!(a, b);

            let str: String = serde_json::to_string(&str_a).unwrap();
            let deser_a: $str_type = serde_json::from_str(&str).unwrap();
            assert_eq!(a, deser_a.0);
        };
    }

    #[test]
    fn test_u128() {
        test_serde!(U128, u128, 0);
        test_serde!(U128, u128, 1);
        test_serde!(U128, u128, 123);
        test_serde!(U128, u128, 10u128.pow(18));
        test_serde!(U128, u128, 2u128.pow(100));
        test_serde!(U128, u128, u128::max_value());
    }

    #[test]
    fn test_i128() {
        test_serde!(I128, i128, 0);
        test_serde!(I128, i128, 1);
        test_serde!(I128, i128, -1);
        test_serde!(I128, i128, 123);
        test_serde!(I128, i128, 10i128.pow(18));
        test_serde!(I128, i128, 2i128.pow(100));
        test_serde!(I128, i128, -(2i128.pow(100)));
        test_serde!(I128, i128, i128::max_value());
        test_serde!(I128, i128, i128::min_value());
    }

    #[test]
    fn test_u64() {
        test_serde!(U64, u64, 0);
        test_serde!(U64, u64, 1);
        test_serde!(U64, u64, 123);
        test_serde!(U64, u64, 10u64.pow(18));
        test_serde!(U64, u64, 2u64.pow(60));
        test_serde!(U64, u64, u64::max_value());
    }

    #[test]
    fn test_i64() {
        test_serde!(I64, i64, 0);
        test_serde!(I64, i64, 1);
        test_serde!(I64, i64, -1);
        test_serde!(I64, i64, 123);
        test_serde!(I64, i64, 10i64.pow(18));
        test_serde!(I64, i64, 2i64.pow(60));
        test_serde!(I64, i64, -(2i64.pow(60)));
        test_serde!(I64, i64, i64::max_value());
        test_serde!(I64, i64, i64::min_value());
    }
}
