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
