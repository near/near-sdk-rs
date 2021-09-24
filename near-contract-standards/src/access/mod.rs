use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, require, AccountId};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Ownable {
    pub owner: AccountId,
    pub null_owner: AccountId,
}

impl Ownable {
    pub fn new() -> Self {
        Self { owner: env::predecessor_account_id(), null_owner: "".parse::<AccountId>().unwrap() }
    }

    pub fn owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn only_owner(&self) {
        require!(env::predecessor_account_id() == self.owner(), "Ownable: caller is not the owner");
    }

    pub fn renounce_ownership(&mut self) {
        self.only_owner();
        self.owner = self.null_owner.clone();
    }

    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.only_owner();
        require!(new_owner != self.null_owner, "Ownable: new owner is undefined");
        self.owner = new_owner;
    }
}
