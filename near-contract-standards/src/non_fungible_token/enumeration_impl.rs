use crate::non_fungible_token::core_impl::NonFungibleToken;
use crate::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_sdk::json_types::{U128, ValidAccountId};
use crate::non_fungible_token::token::{Token, TokenId};
use std::collections::HashMap;
use near_sdk::AccountId;
use near_sdk::log;

impl NonFungibleToken {
    // Helper function used by a enumerations methods
    // Note: this method is not exposed publicly to end users
    pub fn enum_get_token(&self, owner_id: AccountId, token_id: TokenId) -> Token {
        let metadata = self.token_metadata_by_id.as_ref().unwrap().get(&token_id);
        let approved_account_ids = self
            .approvals_by_id.as_ref()
            .unwrap().get(&token_id).or_else(|| Some(HashMap::new()));

        Token {
            token_id,
            owner_id,
            metadata,
            approved_account_ids
        }
    }
}

impl NonFungibleTokenEnumeration for NonFungibleToken {
    fn nft_total_supply(self) -> U128 {
        // An unfortunate cast from the max of TreeMap to the spec
        (self.owner_by_id.len() as u128).into()
    }

    fn nft_tokens(&self, from_index: Option<String>, limit: Option<u64>) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        // Get starting index, whether or not it was explicitly given
        let start_index = if from_index.is_none() {
            // "0" according to the spec here:
            // https://nomicon.io/Standards/NonFungibleToken/Enumeration.html#interface
            "0".to_string() as TokenId
        } else {
            // Since TreeMap's iter_from is exclusive to the start index,
            // use key below it or minimum key
            let from_index = from_index.unwrap();
            from_index
        };

        if !self.owner_by_id.contains_key(&start_index) { return vec![] }
        let mut token_ids: Vec<TokenId> = vec![start_index.clone()];
        let iterate_tokens = self.owner_by_id.iter_from(start_index);
        let following_entries: Vec<TokenId> = if let Some(limit) = limit {
            iterate_tokens.take((limit - 1) as usize).map(|(token_id, _)| token_id).collect()
        } else {
            iterate_tokens.map(|(token_id, _)| token_id).collect()
        };
        token_ids.extend_from_slice(&following_entries);

        for token_id in token_ids {
            let owner_id = self.owner_by_id.get(&token_id).unwrap();
            let token = self.enum_get_token(owner_id.clone(), token_id);
            tokens.push(token);
        }
        tokens
    }

    fn nft_supply_for_owner(self, account_id: ValidAccountId) -> U128 {
        let tokens_per_owner = self.tokens_per_owner.expect("Could not find tokens_per_owner when calling a method on the enumeration standard.");
        if let Some(account_tokens) = tokens_per_owner.get((&account_id).as_ref()) {
            U128::from(account_tokens.len() as u128)
        } else {
            U128::from(0)
        }
    }

    fn nft_tokens_for_owner(&self, account_id: ValidAccountId, from_index: Option<TokenId>, limit: Option<u64>) -> Vec<Token> {
        let tokens_per_owner = self.tokens_per_owner.as_ref().expect("Could not find tokens_per_owner when calling a method on the enumeration standard.");
        let token_set = tokens_per_owner.get(&account_id.as_ref());
        if token_set.is_none() { return vec![] }
        let mut tokens: Vec<Token> = Vec::new();

        let has_limit = limit.is_some();
        if has_limit {
            assert_ne!(limit.unwrap(), 0, "limit must be non-zero.")
        }
        let mut decrementing_limit = if has_limit { limit.unwrap() } else { 0 };
        let has_from_index = from_index.clone().is_some();
        let from_index_val = if has_from_index { from_index.unwrap() } else { "".to_string() };

        // Not-great way to iterate through an UnorderedSet
        for token_id in token_set.unwrap().iter() {
            // When there's a designated `from_index` to start at
            if has_from_index {
                if tokens.is_empty() && has_from_index && token_id == from_index_val {
                    // Push tokens to return vector
                    let token = self.enum_get_token(account_id.as_ref().clone(), token_id.clone());
                    tokens.push(token);
                } else if !tokens.is_empty() {
                    // Push tokens to return vector
                    let token = self.enum_get_token(account_id.as_ref().clone(), token_id.clone());
                    tokens.push(token);
                }
            } else {
                // When there's no `from_index` just push the token
                let token = self.enum_get_token(account_id.as_ref().clone(), token_id.clone());
                tokens.push(token)
            }
            if !tokens.is_empty() && has_limit {
                decrementing_limit -= 1;
                if decrementing_limit == 0 { break; }
            }
        }

        tokens
    }
}