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
        let this = Self { accounts: temp_accounts };
        this.validate();
        this
    }

    pub(crate) fn validate(&self) {
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
        let mut payout = Payout {
            payout: self
                .accounts
                .iter()
                .map(|(account, percent)| (account.clone(), apply_percent(percent, balance).into()))
                .collect(),
        };
        let rest = balance - payout.payout.values().fold(0, |acc, &sum| acc + sum.0);
        let owner_payout: u128 = payout.payout.get(owner_id).map_or(0, |x| x.0) + rest;
        payout.payout.insert(owner_id.clone(), owner_payout.into());
        payout
    }
}

// TODO: Perhaps redo this function so that it never overflows.
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
#[cfg(test)]
mod tests {
    use crate::non_fungible_token::payout::payout_impl::apply_percent;
    use crate::non_fungible_token::payout::Royalties;
    use near_sdk::collections::TreeMap;
    use near_sdk::json_types::U128;
    use near_sdk::{AccountId, Balance};
    use std::mem;

    const KEY_PREFIX: &[u8] = "test_prefix".as_bytes();

    #[test]
    fn validate_happy_path() {
        let mut map = TreeMap::new(KEY_PREFIX);

        // Works with up to 100% and at most 10 accounts.
        for idx in 0..10 {
            map.insert(&AccountId::new_unchecked(format!("bob_{}", idx)), &10);
        }

        let mut royalties = Royalties::new(KEY_PREFIX);

        mem::swap(&mut royalties.accounts, &mut map);
        royalties.validate();

        // Make sure that max royalties works.
        let owner_id = AccountId::new_unchecked("alice".to_string());
        let payout = royalties.create_payout(1000, &owner_id);
        for (key, value) in payout.payout.iter() {
            map.contains_key(key);
            if *key == owner_id {
                assert_eq!(*value, U128::from(0));
            } else {
                assert_eq!(*value, U128::from(apply_percent(10, 1000)));
            }
        }
    }

    #[test]
    fn validate_owner_rest_path() {
        let mut map = TreeMap::new(KEY_PREFIX);

        for idx in 0..10 {
            map.insert(&AccountId::new_unchecked(format!("bob_{}", idx)), &8);
        }

        let mut royalties = Royalties::new(KEY_PREFIX);

        mem::swap(&mut royalties.accounts, &mut map);
        royalties.validate();

        // Make sure we don't overflow and don't end up with mismatched results due to using int as
        // opposed to float.
        let balance = Balance::MAX / 10_000 * 100;
        let owner_id = AccountId::new_unchecked("alice".to_string());
        let payout = royalties.create_payout(balance, &owner_id);
        for (key, value) in payout.payout.iter() {
            map.contains_key(key);
            if *key == owner_id {
                assert_eq!(*value, U128::from(apply_percent(20, balance)));
            } else {
                assert_eq!(*value, U128::from(apply_percent(8, balance)));
            }
        }
    }

    #[test]
    fn validate_owner_rest_and_royalty_path() {
        let mut map = TreeMap::new(KEY_PREFIX);

        for idx in 0..9 {
            map.insert(&AccountId::new_unchecked(format!("bob_{}", idx)), &8);
        }
        map.insert(&AccountId::new_unchecked("alice".to_string()), &8);

        let mut royalties = Royalties::new(KEY_PREFIX);

        mem::swap(&mut royalties.accounts, &mut map);
        royalties.validate();

        // Make sure we don't overflow and don't end up with mismatched results due to using int as
        // opposed to float.
        let balance = Balance::MAX / 10_000 * 100;
        let owner_id = AccountId::new_unchecked("alice".to_string());
        let payout = royalties.create_payout(balance, &owner_id);
        for (key, value) in payout.payout.iter() {
            map.contains_key(key);
            if *key == owner_id {
                assert_eq!(*value, U128::from(apply_percent(28, balance)));
            } else {
                assert_eq!(*value, U128::from(apply_percent(8, balance)));
            }
        }
    }

    #[test]
    fn validate_empty_inputs() {
        let _ = Royalties::new(KEY_PREFIX);
    }

    #[test]
    #[should_panic(expected = "royalty overflow")]
    fn create_payout_overflow() {
        let mut map = TreeMap::new(KEY_PREFIX);

        for idx in 0..10 {
            map.insert(&AccountId::new_unchecked(format!("bob_{}", idx)), &10);
        }

        let royalties = Royalties::new(KEY_PREFIX);

        royalties.create_payout(Balance::MAX, &AccountId::new_unchecked("alice".to_string()));
    }

    #[test]
    #[should_panic(expected = "can only have a maximum of 10 accounts spliting royalties")]
    fn validate_too_many_accounts() {
        let mut map = TreeMap::new(KEY_PREFIX);

        // Fails with 11 accounts.
        for idx in 0..11 {
            map.insert(&AccountId::new_unchecked(format!("bob_{}", idx)), &10);
        }

        let mut royalties = Royalties::new(KEY_PREFIX);

        mem::swap(&mut royalties.accounts, &mut map);
        royalties.validate();
    }

    #[test]
    #[should_panic(expected = "each royalty should be at most 100")]
    fn validate_royalty_per_account_fails() {
        let mut map = TreeMap::new(KEY_PREFIX);

        // Fails with more than 100% per account.
        map.insert(&AccountId::new_unchecked("bob".to_string()), &101);

        let mut royalties = Royalties::new(KEY_PREFIX);

        mem::swap(&mut royalties.accounts, &mut map);
        royalties.validate();
    }

    #[test]
    #[should_panic(expected = "total percent of each royalty split must be at most 100")]
    fn validate_total_royalties_fails() {
        let mut map = TreeMap::new(KEY_PREFIX);

        // Fails with total royalties over 100%.
        for idx in 0..10 {
            map.insert(&AccountId::new_unchecked(format!("bob_{}", idx)), &11);
        }
        let mut royalties = Royalties::new(KEY_PREFIX);

        mem::swap(&mut royalties.accounts, &mut map);
        royalties.validate();
    }
}
