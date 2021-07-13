use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Represents the amount of NEAR tokens in "gas units" which are used to fund transactions.
#[derive(
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
pub struct Gas(u64);

impl Gas {
    pub const fn new(amount: u64) -> Self {
        Self(amount)
    }
}

impl Serialize for Gas {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
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

impl core::ops::Add for Gas {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl core::ops::AddAssign for Gas {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl core::ops::SubAssign for Gas {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl core::ops::Sub for Gas {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl core::ops::Add<u64> for Gas {
    type Output = Self;

    fn add(self, other: u64) -> Self {
        Self(self.0 + other)
    }
}

impl core::ops::Sub<u64> for Gas {
    type Output = Self;

    fn sub(self, other: u64) -> Self {
        Self(self.0 - other)
    }
}

impl core::ops::Mul<u64> for Gas {
    type Output = Self;

    fn mul(self, other: u64) -> Self {
        Self(self.0 * other)
    }
}

impl core::ops::Div<u64> for Gas {
    type Output = Self;

    fn div(self, other: u64) -> Self {
        Self(self.0 / other)
    }
}
