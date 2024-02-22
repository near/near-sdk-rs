use near_sdk::NearSchema;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
    NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Resource {
    Battery,
    RgbSensor,
    ThermalSensor,
    PoseEstimation,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
    NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Reward {
    Score,
    Token,
    Prediction,
    Currency,
    Policy,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
    NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Asset {
    Resource(Resource),
    Reward(Reward),
    MissionTime,
    Trust,
}

#[derive(
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    PartialOrd,
    Ord,
    BorshDeserialize,
    BorshSerialize,
    NearSchema
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Exchange {
    MissionTimeWithResource,
    MissionTimeWithTrust,
}
