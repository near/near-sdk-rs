// Find all our documentation at https://docs.near.org
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near;
use near_sdk::contract_error;

#[contract_error]
enum MyErrorEnum {X}
impl AsRef<str> for MyErrorEnum {
    fn as_ref(&self) -> &str {
        match self {
            MyErrorEnum::X => "X",
        }
    }
}

#[contract_error]
struct MyErrorStruct {
    x: u32,
}

#[derive(Debug)]
#[near(serializers=[json])]
enum AnotherError {X}
impl AsRef<str> for AnotherError {
    fn as_ref(&self) -> &str {
        match self {
            AnotherError::X => "X",
        }
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
    pub fn inc_handle_result(&mut self, is_error: bool) -> Result<u32, AnotherError> {
        self.value += 1;
        if is_error {
            return Err(AnotherError::X);
        } else {
            return Ok(self.value);
        }
    }

    #[persist_on_error]
    pub fn inc_persist_on_error(&mut self, is_error: bool) -> Result<u32, MyErrorEnum> {
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

