use super::NonFungibleTokenPayout;
use crate::non_fungible_token::core::NonFungibleTokenCore;
use crate::non_fungible_token::payout::*;
use crate::non_fungible_token::NonFungibleToken;
use near_sdk::{assert_one_yocto, env};
use near_sdk::{require, AccountId, Balance, IntoStorageKey};

impl Royalties {
    pub fn new<S>(key_prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        let temp_accounts: TreeMap<AccountId, BasisPoint> = TreeMap::new(key_prefix);
        let this = Self { accounts: temp_accounts, percent: 0 };
        this.validate();
        this
    }

    pub(crate) fn validate(&self) {
        require!(self.percent <= 100, "royalty percent must be between 0 - 100");
        require!(
            self.accounts.len() <= 10,
            "can only have a maximum of 10 accounts spliting royalties"
        );
        let mut total: BasisPoint = 0;
        self.accounts.iter().for_each(|(_, percent)| {
            require!(percent <= 100, "each royalty should be at most 100");
            total += percent;
        });
        require!(total <= 100, "total percent of each royalty split must be at most 100")
    }

    pub fn create_payout(&self, balance: Balance, owner_id: &AccountId) -> Payout {
        let royalty_payment = apply_percent(self.percent, balance);
        let mut payout = Payout {
            payout: self
                .accounts
                .iter()
                .map(|(account, percent)| {
                    (account.clone(), apply_percent(percent, royalty_payment).into())
                })
                .collect(),
        };
        let rest = balance - royalty_payment;
        let owner_payout: u128 = payout.payout.get(owner_id).map_or(0, |x| x.0) + rest;
        payout.payout.insert(owner_id.clone(), owner_payout.into());
        payout
    }
}

fn apply_percent(percent: BasisPoint, int: u128) -> u128 {
    int.checked_mul(percent as u128).unwrap_or_else(|| env::panic_str("royalty overflow")) / 100u128
}

impl NonFungibleTokenPayout for NonFungibleToken {
    fn nft_payout(&self, token_id: String, balance: U128, max_len_payout: Option<u32>) -> Payout {
        let owner_id = self.owner_by_id.get(&token_id).expect("No such token_id");
        let payout = self
            .royalties
            .as_ref()
            .map_or(Payout::default(), |r| r.create_payout(balance.0, &owner_id));

        if let Some(max_len_payout) = max_len_payout {
            require!(
                payout.payout.len() <= max_len_payout as usize,
                "payout number can't exceed `max_len_payout`"
            );
        }

        payout
    }

    fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
        balance: U128,
        max_len_payout: Option<u32>,
    ) -> Payout {
        assert_one_yocto();
        let payout = self.nft_payout(token_id.clone(), balance, max_len_payout);
        self.nft_transfer(receiver_id, token_id, approval_id, memo);
        payout
    }
}
