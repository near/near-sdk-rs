#[macro_export]
macro_rules! impl_fungible_token_core {
    ($contract: ident, $token: ident, $on_tokens_burned_block: block,) => {
        use near_contract_standards::fungible_token::core::FungibleTokenCore;
        use near_contract_standards::fungible_token::resolver::FungibleTokenResolver;

        #[near_bindgen]
        impl FungibleTokenCore for $contract {
            #[payable]
            fn ft_transfer(
                &mut self,
                receiver_id: ValidAccountId,
                amount: U128,
                memo: Option<String>,
            ) {
                self.$token.ft_transfer(receiver_id, amount, memo)
            }

            #[payable]
            fn ft_transfer_call(
                &mut self,
                receiver_id: ValidAccountId,
                amount: U128,
                memo: Option<String>,
                msg: String,
            ) -> PromiseOrValue<U128> {
                self.$token.ft_transfer_call(receiver_id, amount, memo, msg)
            }

            fn ft_total_supply(&self) -> U128 {
                self.$token.ft_total_supply()
            }

            fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
                self.$token.ft_balance_of(account_id)
            }
        }

        #[near_bindgen]
        impl FungibleTokenResolver for $contract {
            #[private]
            fn ft_resolve_transfer(
                &mut self,
                sender_id: ValidAccountId,
                receiver_id: ValidAccountId,
                amount: U128,
            ) -> U128 {
                let sender_id: AccountId = sender_id.into();
                let (used_amount, burned_amount) =
                    self.$token.internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
                if burned_amount > 0 {
                    $on_tokens_burned_block
                }
                used_amount.into()
            }
        }
    };
    ($contract: ident, $token: ident, $on_tokens_burned: ident) => {
        near_contract_standards::impl_fungible_token_core!($contract, $token, {
            self.$on_tokens_burned(sender_id, burned_amount);
        },);
    };
    ($contract: ident, $token: ident) => {
        near_contract_standards::impl_fungible_token_core!($contract, $token, {},);
    };
}
