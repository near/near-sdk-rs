use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use serde::{Deserialize, Serialize};

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
    Deserialize,
    Serialize,
    Hash,
    BorshSchema,
)]
pub struct Gas(u64);

impl Gas {
    pub const fn new(amount: u64) -> Self {
        Self(amount)
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
