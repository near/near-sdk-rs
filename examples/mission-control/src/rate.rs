use crate::account::*;
use crate::asset::*;
use near_sdk::near;
use std::collections::HashMap;

#[derive(PartialEq, Eq)]
#[near(serializers = [json, borsh])]
pub struct Rate {
    pub credit: HashMap<Asset, Quantity>,
    pub debit: HashMap<Asset, Quantity>,
}

impl Rate {}
