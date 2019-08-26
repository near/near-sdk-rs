use crate::account::*;
use crate::asset::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct Rate {
    pub credit: HashMap<Asset, Quantity>,
    pub debit: HashMap<Asset, Quantity>,
}

impl Rate {}
