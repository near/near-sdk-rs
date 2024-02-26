use crate::account::*;
use crate::asset::*;
use std::collections::HashMap;
use near_sdk::near;

#[derive(PartialEq, Eq)]
#[near(serializers = [json, borsh])]
pub struct Rate {
    pub credit: HashMap<Asset, Quantity>,
    pub debit: HashMap<Asset, Quantity>,
}

impl Rate {}
