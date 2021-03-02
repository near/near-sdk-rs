#[macro_export]
macro_rules! impl_fungible_token_core {
    ($contract: ident, $token: ident) => {
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
                self.$token.ft_resolve_transfer(sender_id, receiver_id, amount)
            }
        }

        #[near_bindgen]
        impl AccountRegistrar for $contract {
            #[payable]
            fn ar_register(&mut self, account_id: Option<String>, msg: Option<String>) -> bool {
                self.$token.ar_register(account_id, msg)
            }

            fn ar_is_registered(&self, account_id: String) -> bool {
                self.$token.ar_is_registered(account_id)
            }

            #[payable]
            fn ar_unregister(&mut self, force: Option<bool>) -> bool {
                self.$token.ar_unregister(force)
            }

            fn ar_registration_fee(&self) -> U128 {
                self.$token.ar_registration_fee()
            }
        }
    };
}

#[macro_export]
macro_rules! impl_fungible_token_storage {
    ($contract: ident, $token: ident) => {
        use near_contract_standards::storage_manager::{AccountStorageBalance, StorageManager};

        #[near_bindgen]
        impl StorageManager for $contract {
            #[payable]
            fn storage_deposit(
                &mut self,
                account_id: Option<ValidAccountId>,
            ) -> AccountStorageBalance {
                self.$token.storage_deposit(account_id)
            }

            #[payable]
            fn storage_withdraw(&mut self, amount: Option<U128>) -> AccountStorageBalance {
                self.$token.storage_withdraw(amount)
            }

            fn storage_minimum_balance(&self) -> U128 {
                self.$token.storage_minimum_balance()
            }

            fn storage_balance_of(&self, account_id: ValidAccountId) -> AccountStorageBalance {
                self.$token.storage_balance_of(account_id)
            }
        }
    };
}
