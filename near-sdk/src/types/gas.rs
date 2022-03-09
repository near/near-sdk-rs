use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use core::ops;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Represents the amount of NEAR tokens in "gas units" which are used to fund transactions.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Hash,
    BorshSchema,
)]
#[repr(transparent)]
pub struct Gas(pub u64);

impl Gas {
    /// One Tera gas, which is 10^12 gas units.
    pub const ONE_TERA: Gas = Gas(1_000_000_000_000);
}

impl Serialize for Gas {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buf = [0u8; 20];
        let remainder = {
            use std::io::Write;

            let mut w: &mut [u8] = &mut buf;
            write!(w, "{}", self.0).unwrap_or_else(|_| crate::env::abort());
            w.len()
        };
        let len = buf.len() - remainder;

        let s = std::str::from_utf8(&buf[..len]).unwrap_or_else(|_| crate::env::abort());
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Gas {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse::<u64>().map(Self).map_err(|err| de::Error::custom(err.to_string()))
    }
}

impl From<u64> for Gas {
    fn from(amount: u64) -> Self {
        Self(amount)
    }
}

impl From<Gas> for u64 {
    fn from(gas: Gas) -> Self {
        gas.0
    }
}

impl ops::Add for Gas {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl ops::AddAssign for Gas {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl ops::SubAssign for Gas {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl ops::Sub for Gas {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl ops::Mul<u64> for Gas {
    type Output = Self;

    fn mul(self, other: u64) -> Self {
        Self(self.0 * other)
    }
}

impl ops::Div<u64> for Gas {
    type Output = Self;

    fn div(self, other: u64) -> Self {
        Self(self.0 / other)
    }
}

impl ops::Rem<u64> for Gas {
    type Output = Self;

    fn rem(self, rhs: u64) -> Self::Output {
        Self(self.0.rem(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_json_ser(val: u64) {
        let gas = Gas(val);
        let ser = serde_json::to_string(&gas).unwrap();
        assert_eq!(ser, format!("\"{}\"", val));
        let de: Gas = serde_json::from_str(&ser).unwrap();
        assert_eq!(de.0, val);
    }

    #[test]
    fn json_ser() {
        test_json_ser(u64::MAX);
        test_json_ser(8);
        test_json_ser(0);
    }
}
