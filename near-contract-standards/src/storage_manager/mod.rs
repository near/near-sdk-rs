use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::Serialize;

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountStorageBalance {
    pub total: U128,
    pub available: U128,
}

pub trait StorageManager {
    fn storage_deposit(&mut self, account_id: Option<ValidAccountId>) -> AccountStorageBalance;

    fn storage_withdraw(&mut self, amount: Option<U128>) -> AccountStorageBalance;

    fn storage_minimum_balance(&self) -> U128;

    fn storage_balance_of(&self, account_id: ValidAccountId) -> AccountStorageBalance;
}
