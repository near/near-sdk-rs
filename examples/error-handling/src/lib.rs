// Find all our documentation at https://docs.near.org
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near;
use near_sdk::contract_error;
use near_sdk::FunctionError;

#[contract_error]
enum MyErrorEnum {X}

#[contract_error]
struct MyErrorStruct {
    x: u32,
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
    pub fn inc_handle_result(&mut self, is_error: bool) -> Result<u32, &'static str> {
        self.value += 1;
        if is_error {
            return Err("error in inc_handle_result");
        } else {
            return Ok(self.value);
        }
    }

    #[persist_on_error]
    pub fn inc_persist_on_e(&mut self, is_error: bool) -> Result<u32, MyErrorEnum> {
        self.value += 1;
        if is_error {
            return Err(MyErrorEnum::X);
        } else {
            return Ok(self.value);
        }
    }

    pub fn inc_just_result(&mut self, is_error: bool) -> Result<u32, MyErrorStruct> {
        self.value += 1;
        if is_error {
            return Err(MyErrorStruct { x: 5 });
        } else {
            return Ok(self.value);
        }
    }

    pub fn inc_just_simple(&mut self, is_error: bool) -> u32 {
        self.value += 1;
        if is_error {
            ::near_sdk::env::panic_str("Error");
        } else {
            return self.value;
        }
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }   
}

