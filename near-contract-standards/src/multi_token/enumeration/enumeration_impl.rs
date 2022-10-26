use near_sdk::json_types::U128;
use near_sdk::{require, AccountId};

use crate::multi_token::{
    core::MultiToken,
    token::{Token, TokenId},
};

use super::MultiTokenEnumeration;

impl MultiToken {
    fn enum_get_token(&self, owner_id: AccountId, token_id: TokenId) -> Token {
        let metadata = self.token_metadata_by_id.as_ref().unwrap().get(&token_id);
        let supply = self.total_supply.get(&token_id).unwrap();

        Token { token_id, owner_id, metadata, supply }
    }
}

impl MultiTokenEnumeration for MultiToken {
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        let from_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            self.owner_by_id.len() as u128 > from_index,
            "Out of bounds, please use a smaller from_index."
        );
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Limit cannot be 0");

        self.owner_by_id
            .iter()
            .skip(from_index as usize)
            .take(limit as usize)
            .map(|(token_id, owner_id)| self.enum_get_token(owner_id, token_id))
            .collect()
    }

    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        let tokens_per_owner = self.tokens_per_owner.as_ref().expect("Could not find field");
        let token_set = if let Some(set) = tokens_per_owner.get(&account_id) {
            set
        } else {
            return vec![];
        };

        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Limit cannot be 0");
        let from_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            token_set.len() as u128 > from_index,
            "Out of bounds, please use a smaller from_index."
        );

        token_set
            .iter()
            .skip(from_index as usize)
            .take(limit as usize)
            .map(|token_id| self.enum_get_token(account_id.clone(), token_id))
            .collect()
    }
}
