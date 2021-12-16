use crate::non_fungible_token::payout::Payout;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    require,
    serde::{Deserialize, Serialize},
    AccountId,
    Balance,
};

use std::collections::HashMap;

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct Royalties {
    pub accounts: HashMap<AccountId, u8>,
    pub percent: u8,
}

impl Royalties {
    pub fn new(accounts: HashMap<AccountId, u8>, percent: u8) -> Self {
        let this = Self { accounts, percent };
        this.validate();
        this
    }
    pub(crate) fn validate(&self) {
        require!(self.percent <= 100, "royalty percent must be between 0 - 100");
        require!(
            self.accounts.len() <= 10,
            "can only have a maximum of 10 accounts spliting royalties"
        );
        let mut total: u8 = 0;
        self.accounts.iter().for_each(|(_, percent)| {
            require!(*percent <= 100, "each royalty should be less than 100");
            total += percent;
        });
        require!(total <= 100, "total percent of each royalty split  must be less than 100")
    }
    pub fn create_payout(&self, balance: Balance, owner_id: &AccountId) -> Payout {
        let royalty_payment = apply_percent(self.percent, balance);
        let mut payout = Payout {
            payout: self
                .accounts
                .iter()
                .map(|(account, percent)| {
                    (account.clone(), apply_percent(*percent, royalty_payment).into())
                })
                .collect(),
        };
        let rest = balance - royalty_payment;
        let owner_payout: u128 = payout.payout.get(owner_id).map_or(0, |x| x.0) + rest;
        payout.payout.insert(owner_id.clone(), owner_payout.into());
        payout
    }
}

fn apply_percent(percent: u8, int: u128) -> u128 {
    int * percent as u128 / 100u128
}
