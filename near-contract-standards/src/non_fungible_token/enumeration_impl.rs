use crate::non_fungible_token::core_impl::NonFungibleToken;
use crate::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use crate::non_fungible_token::token::Token;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::AccountId;
use std::collections::HashMap;

type TokenId = String;

impl NonFungibleToken {
    /// Helper function used by a enumerations methods
    /// Note: this method is not exposed publicly to end users
    pub fn enum_get_token(&self, owner_id: AccountId, token_id: TokenId) -> Token {
        let metadata = self.token_metadata_by_id.as_ref().unwrap().get(&token_id);
        let approved_account_ids =
            self.approvals_by_id.as_ref().unwrap().get(&token_id).or_else(|| Some(HashMap::new()));

        Token { token_id, owner_id, metadata, approved_account_ids }
    }
}

impl NonFungibleTokenEnumeration for NonFungibleToken {
    fn nft_total_supply(self) -> U128 {
        // An unfortunate cast from the max of TreeMap to the spec
        (self.owner_by_id.len() as u128).into()
    }

    fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        // Get starting index, whether or not it was explicitly given
        let start_index: u128 = if from_index.is_none() {
            // "0" according to the spec here:
            // https://nomicon.io/Standards/NonFungibleToken/Enumeration.html#interface
            0
        } else {
            // Since TreeMap's iter_from is exclusive to the start index,
            // use key below it or minimum key
            from_index.unwrap().0
        };

        assert!(
            (self.owner_by_id.len() as u128) > start_index,
            "Out of bounds, please use a smaller from_index."
        );
        let has_limit = limit.is_some();
        let mut decrementing_limit = if has_limit {
            let limit_val = limit.unwrap();
            assert_ne!(limit_val, 0, "Cannot provide limit of 0.");
            limit_val
        } else {
            0
        };

        for (token_id, owner_id) in self.owner_by_id.iter().skip(start_index as usize) {
            tokens.push(self.enum_get_token(owner_id, token_id));
            if has_limit {
                decrementing_limit -= 1;
                if decrementing_limit == 0 {
                    break;
                }
            }
        }

        tokens
    }

    fn nft_supply_for_owner(self, account_id: ValidAccountId) -> U128 {
        let tokens_per_owner = self.tokens_per_owner.expect(
            "Could not find tokens_per_owner when calling a method on the enumeration standard.",
        );
        if let Some(account_tokens) = tokens_per_owner.get((&account_id).as_ref()) {
            U128::from(account_tokens.len() as u128)
        } else {
            U128::from(0)
        }
    }

    fn nft_tokens_for_owner(
        &self,
        account_id: ValidAccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        let tokens_per_owner = self.tokens_per_owner.as_ref().expect(
            "Could not find tokens_per_owner when calling a method on the enumeration standard.",
        );
        let token_set = tokens_per_owner.get(&account_id.as_ref());
        if token_set.is_none() {
            return vec![];
        }

        let has_limit = limit.is_some();
        if has_limit {
            assert_ne!(limit.unwrap(), 0, "limit must be non-zero.")
        }
        let mut decrementing_limit = if has_limit { limit.unwrap() } else { 0 };
        let has_from_index = from_index.clone().is_some();
        let from_index_val: u128 = if has_from_index { from_index.unwrap().0 } else { 0 };

        for token_id in token_set.unwrap().iter().skip(from_index_val as usize) {
            tokens.push(self.enum_get_token(account_id.as_ref().parse().unwrap(), token_id));
            if has_limit {
                decrementing_limit -= 1;
                if decrementing_limit == 0 {
                    break;
                }
            }
        }

        tokens
    }
}
