use crate::account::*;
use crate::asset::*;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Rate {
    pub credit: BTreeMap<Asset, Quantity>,
    pub debit: BTreeMap<Asset, Quantity>,
}

impl Rate {}
