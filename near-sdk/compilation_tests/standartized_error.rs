//! Regular smart contract.

use near_sdk::near;
use near_sdk::check_trait;
use near_sdk::MyContractError;

#[derive(MyContractError)]
enum MyError {}


#[near(contract_state)]
#[derive(Default)]
struct Incrementer {
    value: u32,
}

#[near]
impl Incrementer {
    #[handle_result]
    pub fn inc(&mut self) -> Result<String, MyError> {
        self.value += 1;
        // let _ = check_trait as fn(& String) ;
        Ok("ok".to_string())
    }

    pub fn get(&mut self) -> Result<String, MyError> {
        self.value += 1;
        Ok("hey".to_string())
    }

    pub fn top(&mut self) {
        self.value += 1;
    }
}

fn main() {}