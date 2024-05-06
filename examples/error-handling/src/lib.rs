// Find all our documentation at https://docs.near.org
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near;
use near_sdk::check_trait;
use near_sdk::MyContractError;

#[derive(MyContractError)]
enum MyError {}

impl AsRef<str> for MyError {
    fn as_ref(&self) -> &str {
        "Not enough balance"
    }
}

// Define the contract structure
#[near(contract_state)]
#[derive(Default)]
pub struct Contract {
    value: u32,
}

// Implement the contract structure
#[near]
impl Contract {
    #[handle_result]
    pub fn inc(&mut self) -> Result<String, MyError> {
        self.value += 1;
        // let _ = check_trait as fn(& String) ;
        Ok("ok".to_string())
    }

    #[persist_on_error]
    pub fn get_my(&mut self) -> Result<String, MyError> {
        self.value += 1;
        Ok("hey".to_string())
    }

    pub fn get_value(&self) -> Result<String, MyError> {
        self.value;
        Ok("ok".to_string())
    }

    pub fn top(&mut self) {
        self.value += 1;
    }
}

