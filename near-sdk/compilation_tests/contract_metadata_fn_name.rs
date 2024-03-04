use near_sdk::near_bindgen;

#[near(contract_state)]
struct Contract {}

#[near]
impl Contract {
    pub fn contract_source_metadata() {}
}

fn main() {}
