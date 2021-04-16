#[macro_export]
macro_rules! impl_non_fungible_token_core {
    ($contract: ident, $token: ident) => {
        use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
        use near_contract_standards::non_fungible_token::resolver::NonFungibleTokenResolver;

        #[near_bindgen]
        impl NonFungibleTokenCore for $contract {
            #[payable]
            fn nft_transfer(
                &mut self,
                receiver_id: ValidAccountId,
                token_id: TokenId,
                approval_id: Option<u64>,
                memo: Option<String>,
            ) {
                self.$token.nft_transfer(receiver_id, token_id, approval_id, memo)
            }

            #[payable]
            fn nft_transfer_call(
                &mut self,
                receiver_id: ValidAccountId,
                token_id: TokenId,
                approval_id: Option<u64>,
                memo: Option<String>,
                msg: String,
            ) -> PromiseOrValue<bool> {
                self.$token.nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
            }

            fn nft_token(self, token_id: TokenId) -> Option<Token> {
                self.$token.nft_token(token_id)
            }

            fn mint(
                &mut self,
                token_id: TokenId,
                token_owner_id: ValidAccountId,
                token_metadata: Option<TokenMetadata>,
            ) -> Token {
                self.$token.mint(token_id, token_owner_id, token_metadata)
            }
        }

        #[near_bindgen]
        impl NonFungibleTokenResolver for $contract {
            #[private]
            fn nft_resolve_transfer(
                &mut self,
                previous_owner_id: AccountId,
                receiver_id: AccountId,
                approved_account_ids: Option<HashMap<AccountId, u64>>,
                token_id: TokenId,
            ) -> bool {
                self.$token.nft_resolve_transfer(
                    previous_owner_id,
                    receiver_id,
                    approved_account_ids,
                    token_id,
                )
            }
        }
    };
}

#[macro_export]
macro_rules! impl_non_fungible_token_approval {
    ($contract: ident, $token: ident) => {
        use near_contract_standards::non_fungible_token::approval::NonFungibleTokenApproval;

        #[near_bindgen]
        impl NonFungibleTokenApproval for $contract {
            fn nft_approve(
                &mut self,
                token_id: TokenId,
                account_id: ValidAccountId,
                msg: Option<String>,
            ) -> Option<Promise> {
                self.$token.nft_approve(token_id, account_id, msg)
            }

            fn nft_revoke(&mut self, token_id: TokenId, account_id: ValidAccountId) {
                self.$token.nft_revoke(token_id, account_id)
            }

            fn nft_revoke_all(&mut self, token_id: TokenId) {
                self.$token.nft_revoke_all(token_id)
            }

            fn nft_is_approved(
                self,
                token_id: TokenId,
                approved_account_id: ValidAccountId,
                approval_id: Option<u64>,
            ) -> bool {
                self.$token.nft_is_approved(token_id, approved_account_id, approval_id)
            }
        }
    };
}
