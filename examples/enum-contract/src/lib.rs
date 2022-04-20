use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, log, near_bindgen, AccountId};

use std::collections::{HashMap, HashSet};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub enum VersionedStatusMessage {
    StatusMessage(DefaultStatusMessage),
    PrimeStatusMessage(PrimeStatusMessage),
}

#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct DefaultStatusMessage {
    records: HashMap<AccountId, String>,
}

#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct PrimeStatusMessage {
    records: HashMap<AccountId, String>,
    whitelist: HashSet<AccountId>,
}

impl Default for VersionedStatusMessage {
    fn default() -> Self {
        VersionedStatusMessage::StatusMessage(DefaultStatusMessage::default())
    }
}

#[near_bindgen]
impl VersionedStatusMessage {
    #[init]
    pub fn new(prime: bool, whitelist: HashSet<AccountId>) -> Self {
        match prime {
            true => VersionedStatusMessage::PrimeStatusMessage(PrimeStatusMessage::new(whitelist)),
            false => VersionedStatusMessage::StatusMessage(DefaultStatusMessage::default()),
        }
    }

    #[payable]
    pub fn set_status(&mut self, message: String) {
        match self {
            VersionedStatusMessage::StatusMessage(status) => status.set_status(message),
            VersionedStatusMessage::PrimeStatusMessage(status) => status.set_status(message),
        }
    }

    pub fn get_status(&self, account_id: AccountId) -> Option<String> {
        match self {
            VersionedStatusMessage::StatusMessage(status) => status.get_status(account_id),
            VersionedStatusMessage::PrimeStatusMessage(status) => status.get_status(account_id),
        }
    }
}

impl DefaultStatusMessage {
    pub fn set_status(&mut self, message: String) {
        let account_id = env::signer_account_id();
        log!("{} set_status with message {}", account_id, message);
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: AccountId) -> Option<String> {
        log!("get_status for account_id {}", account_id);
        self.records.get(&account_id).cloned()
    }
}

impl PrimeStatusMessage {

    pub fn new(whitelist: HashSet<AccountId>) -> Self {
        PrimeStatusMessage {
            whitelist: whitelist.into_iter().collect(),
            ..PrimeStatusMessage::default()
        }
    }

    pub fn set_status(&mut self, message: String) {
        let account_id = env::signer_account_id();
        assert!(
            self.whitelist.contains(&account_id),
            "Account {} is not in the whitelist",
            account_id
        );
        log!("{} set_status with message {}", account_id, message);
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: AccountId) -> Option<String> {
        log!("get_status for account_id {}", account_id);
        self.records.get(&account_id).cloned()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::{bob, alice};
    use near_sdk::test_utils::{get_logs, VMContextBuilder};
    use near_sdk::{testing_env, VMContext};

    fn get_context(is_view: bool, signer_account_id: AccountId) -> VMContext {
        VMContextBuilder::new().signer_account_id(signer_account_id).is_view(is_view).build()
    }

    #[test]
    fn set_get_message() {
        let context = get_context(false, bob());
        testing_env!(context);
        let mut contract = VersionedStatusMessage::default();
        contract.set_status("hello".to_string());
        assert_eq!(get_logs(), vec!["bob.near set_status with message hello"]);
        let context = get_context(true, bob());
        testing_env!(context);
        assert_eq!("hello".to_string(), contract.get_status("bob.near".parse().unwrap()).unwrap());
        assert_eq!(get_logs(), vec!["get_status for account_id bob.near"])
    }

    #[test]
    fn get_nonexistent_message() {
        let context = get_context(true, bob());
        testing_env!(context);
        let contract = VersionedStatusMessage::default();
        assert_eq!(None, contract.get_status("francis.near".parse().unwrap()));
        assert_eq!(get_logs(), vec!["get_status for account_id francis.near"])
    }

    #[test]
    fn set_get_prime_message() {
        let context = get_context(false, bob());
        testing_env!(context);
        let mut contract = VersionedStatusMessage::new(true, HashSet::from([bob()]));
        contract.set_status("hello".to_string());
        assert_eq!(get_logs(), vec!["bob.near set_status with message hello"]);
        let context = get_context(true, bob());
        testing_env!(context);
        assert_eq!("hello".to_string(), contract.get_status("bob.near".parse().unwrap()).unwrap());
        assert_eq!(get_logs(), vec!["get_status for account_id bob.near"]);
    }

    #[test]
    #[should_panic(expected = "Account alice.near is not in the whitelist")]
    fn set_prime_message_not_whitelisted() {
        let context = get_context(false, bob());
        testing_env!(context);
        let mut contract = VersionedStatusMessage::new(true, HashSet::from([bob()]));
        let context = get_context(false, alice());
        testing_env!(context);
        contract.set_status("hello".to_string());
    }
    
    #[test]
    fn get_nonexistent_prime_message() {
        let context = get_context(true, bob());
        testing_env!(context);
        let contract = VersionedStatusMessage::new(true, Default::default());
        assert_eq!(None, contract.get_status("francis.near".parse().unwrap()));
        assert_eq!(get_logs(), vec!["get_status for account_id francis.near"])
    }
}
