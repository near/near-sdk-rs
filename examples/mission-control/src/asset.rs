use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
)]
#[serde(crate = "near_sdk::serde")]
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
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
)]
#[serde(crate = "near_sdk::serde")]
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
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
)]
#[serde(crate = "near_sdk::serde")]
pub enum Asset {
    Resource(Resource),
    Reward(Reward),
    MissionTime,
    Trust,
}

#[derive(
    PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, BorshDeserialize, BorshSerialize,
)]
#[serde(crate = "near_sdk::serde")]
pub enum Exchange {
    MissionTimeWithResource,
    MissionTimeWithTrust,
}
