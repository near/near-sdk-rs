#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests;

use near_sdk::store::LookupMap;
use near_sdk::{AccountId, BorshStorageKey, env, log, near};

#[derive(BorshStorageKey)]
#[near]
struct RecordsKey;

#[near(contract_state)]
pub struct StatusMessage {
    records: LookupMap<AccountId, String>,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self { records: LookupMap::new(RecordsKey) }
    }
}

#[near]
impl StatusMessage {
    #[payable]
    pub fn set_status(&mut self, message: String) {
        let account_id = env::signer_account_id();
        log!("{} set_status with message {}", account_id, message);
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: AccountId) -> Option<&String> {
        log!("get_status for account_id {}", account_id);
        self.records.get(&account_id)
    }
}
