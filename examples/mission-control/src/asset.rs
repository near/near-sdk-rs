use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::schemars::JsonSchema;
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
    JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[schemars(crate = "near_sdk::schemars")]
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
    JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[schemars(crate = "near_sdk::schemars")]
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
    JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[schemars(crate = "near_sdk::schemars")]
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
    JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[schemars(crate = "near_sdk::schemars")]
pub enum Exchange {
    MissionTimeWithResource,
    MissionTimeWithTrust,
}
