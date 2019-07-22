#![feature(const_vec_new)]
use near_bindgen::{near_bindgen, ENV};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
pub struct StatusMessage {
    records: HashMap<Vec<u8>, String>,
}

#[near_bindgen]
impl StatusMessage {
    pub fn set_status(&mut self, message: String) {
        let account_id = ENV.originator_id();
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: Vec<u8>) -> Option<String> {
        self.records.get(&account_id).cloned()
    }
}

#[cfg(feature = "env_test")]
#[cfg(test)]
mod tests {
    use super::*;
    use near_bindgen::{MockedEnvironment, ENV};
    use std::borrow::ToOwned;

    #[test]
    fn set_get_message() {
        ENV.set(Box::new(MockedEnvironment::new()));
        let account_id = b"alice";
        ENV.as_mock().set_originator_id(account_id.to_vec());
        let mut contract = StatusMessage::default();
        contract.set_status("Hello".to_owned());
        assert_eq!(Some("Hello".to_owned()), contract.get_status(account_id.to_vec()));
    }

    #[test]
    fn get_nonexistent_message() {
        ENV.set(Box::new(MockedEnvironment::new()));
        let account_id = b"alice";
        let mut contract = StatusMessage::default();
        assert_eq!(None, contract.get_status(account_id.to_vec()));
    }
}
