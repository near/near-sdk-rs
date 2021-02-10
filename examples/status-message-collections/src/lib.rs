use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LookupSet};
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, near_bindgen};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct StatusMessage {
    pub records: LookupMap<String, String>,
    pub unique_values: LookupSet<String>,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            records: LookupMap::new(b"r".to_vec()),
            unique_values: LookupSet::new(b"s".to_vec()),
        }
    }
}

#[near_bindgen]
impl StatusMessage {
    /// Returns true if the message is unique
    pub fn set_status(&mut self, message: String) -> bool {
        let account_id = env::signer_account_id();
        self.records.insert(&account_id, &message);
        self.unique_values.insert(&message)
    }

    pub fn get_status(&self, account_id: ValidAccountId) -> Option<String> {
        self.records.get(account_id.as_ref())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};
    use std::convert::TryInto;

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new().signer_account_id("bob_near".to_string()).is_view(is_view).build()
    }

    #[test]
    fn set_get_message() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = StatusMessage::default();
        contract.set_status("hello".to_string());
        assert_eq!(
            "hello".to_string(),
            contract.get_status("bob_near".try_into().unwrap()).unwrap()
        );
    }

    #[test]
    fn set_unique_message() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = StatusMessage::default();
        // Unique
        assert!(contract.set_status("hello".to_string()));
        // Unique
        assert!(contract.set_status("hello world".to_string()));
        // Not unique. Same as current
        assert!(!contract.set_status("hello world".to_string()));
        // Not unique. Same as older
        assert!(!contract.set_status("hello".to_string()));
        // Unique
        assert!(contract.set_status("hi".to_string()));
    }

    #[test]
    fn get_nonexistent_message() {
        let context = get_context(true);
        testing_env!(context);
        let contract = StatusMessage::default();
        assert_eq!(None, contract.get_status("francis.near".try_into().unwrap()));
    }
}
