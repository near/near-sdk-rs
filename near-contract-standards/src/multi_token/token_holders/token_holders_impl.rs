use crate::multi_token::{core::MultiToken, token::TokenId};
use near_sdk::{json_types::U128, require, AccountId};

use super::MultiTokenHolders;

impl MultiTokenHolders for MultiToken {
    fn mt_token_holders(
        &self,
        token_id: TokenId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<AccountId> {
        if let Some(holders_per_token) = &self.holders_per_token {
            if let Some(holders) = holders_per_token.get(&token_id) {
                let start_index: u128 = from_index.map(From::from).unwrap_or_default();
                require!(
                    holders.len() as u128 >= start_index,
                    "Out of bounds, please use a smaller from_index."
                );
                let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
                require!(limit != 0, "Limit cannot be 0");

                return holders.iter().skip(start_index as usize).take(limit as usize).collect();
            }
        }

        vec![]
    }
}
