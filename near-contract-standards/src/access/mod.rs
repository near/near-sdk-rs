use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, require, AccountId};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Ownable {
    pub owner: AccountId
}

impl Ownable {
    pub fn new() -> Self {
        Self { owner: env::predecessor_account_id() }
    }

    pub fn owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn only_owner(&self) {
        require!(env::predecessor_account_id() == self.owner(), "Ownable: caller is not the owner");
    }

    pub fn renounce_ownership(&mut self) {
        self.only_owner();
        let null_account_id = "".parse::<AccountId>().unwrap();
        self.owner = null_account_id;
    }

    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.only_owner();
        let null_account_id = "".parse::<AccountId>().unwrap();
        require!(new_owner != null_account_id, "Ownable: new owner is undefined");
        self.owner = new_owner;
    }
}
