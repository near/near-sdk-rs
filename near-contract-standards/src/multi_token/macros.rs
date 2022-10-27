/// The core methods for a basic multi token. Extension standards may be
/// added in addition to this macro.
#[macro_export]
macro_rules! impl_multi_token_core {
    ($contract: ident, $token: ident) => {
        use $crate::multi_token::core::MultiTokenCore;
        use $crate::multi_token::core::MultiTokenResolver;

        #[near_bindgen]
        impl MultiTokenCore for $contract {
            #[payable]
            fn mt_transfer(
                &mut self,
                receiver_id: AccountId,
                token_id: TokenId,
                amount: U128,
                approval: Option<(AccountId, u64)>,
                memo: Option<String>,
            ) {
                self.$token.mt_transfer(receiver_id, token_id, amount, approval, memo)
            }

            #[payable]
            fn mt_batch_transfer(
                &mut self,
                receiver_id: AccountId,
                token_ids: Vec<TokenId>,
                amounts: Vec<U128>,
                approvals: Option<Vec<Option<(AccountId, u64)>>>,
                memo: Option<String>,
            ) {
                self.$token.mt_batch_transfer(receiver_id, token_ids, amounts, approvals, memo)
            }

            #[payable]
            fn mt_transfer_call(
                &mut self,
                receiver_id: AccountId,
                token_id: TokenId,
                amount: U128,
                approval: Option<(AccountId, u64)>,
                memo: Option<String>,
                msg: String,
            ) -> PromiseOrValue<U128> {
                self.$token.mt_transfer_call(receiver_id, token_id, amount, approval, memo, msg)
            }

            #[payable]
            fn mt_batch_transfer_call(
                &mut self,
                receiver_id: AccountId,
                token_ids: Vec<TokenId>,
                amounts: Vec<U128>,
                approvals: Option<Vec<Option<(AccountId, u64)>>>,
                memo: Option<String>,
                msg: String,
            ) -> PromiseOrValue<Vec<U128>> {
                self.$token.mt_batch_transfer_call(
                    receiver_id,
                    token_ids,
                    amounts,
                    approvals,
                    memo,
                    msg,
                )
            }

            fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>> {
                self.$token.mt_token(token_ids)
            }

            fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128 {
                self.$token.mt_balance_of(account_id, token_id)
            }

            fn mt_batch_balance_of(
                &self,
                account_id: AccountId,
                token_ids: Vec<TokenId>,
            ) -> Vec<U128> {
                self.$token.mt_batch_balance_of(account_id, token_ids)
            }

            fn mt_supply(&self, token_id: TokenId) -> Option<U128> {
                self.$token.mt_supply(token_id)
            }

            fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>> {
                self.$token.mt_batch_supply(token_ids)
            }
        }

        #[near_bindgen]
        impl MultiTokenResolver for $contract {
            #[private]
            fn mt_resolve_transfer(
                &mut self,
                previous_owner_ids: Vec<AccountId>,
                receiver_id: AccountId,
                token_ids: Vec<TokenId>,
                amounts: Vec<U128>,
                approvals: Option<Vec<Option<Vec<(AccountId, u64, U128)>>>>,
            ) -> Vec<U128> {
                self.$token.mt_resolve_transfer(
                    previous_owner_ids,
                    receiver_id,
                    token_ids,
                    amounts,
                    approvals,
                )
            }
        }
    };
}

/// Multi token approval management allows for an escrow system where
/// multiple approvals per token exist.
#[macro_export]
macro_rules! impl_multi_token_approval {
    ($contract: ident, $token: ident) => {
        use $crate::multi_token::approval::MultiTokenApproval;

        #[near_bindgen]
        impl MultiTokenApproval for $contract {
            #[payable]
            fn mt_approve(
                &mut self,
                token_ids: Vec<TokenId>,
                amounts: Vec<U128>,
                grantee_id: AccountId,
                msg: Option<String>,
            ) -> Option<Promise> {
                self.$token.mt_approve(token_ids, amounts, grantee_id, msg)
            }

            #[payable]
            fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId) {
                self.$token.mt_revoke(token_ids, account_id)
            }

            #[payable]
            fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>) {
                self.$token.mt_revoke_all(token_ids)
            }

            fn mt_is_approved(
                &self,
                owner_id: AccountId,
                token_ids: Vec<TokenId>,
                approved_account_id: AccountId,
                amounts: Vec<U128>,
                approval_ids: Option<Vec<u64>>,
            ) -> bool {
                self.$token.mt_is_approved(owner_id, token_ids, approved_account_id, amounts, approval_ids)
            }
        }
    };
}

/// Multi-token enumeration adds the extension standard offering several
/// view-only methods to get token supply, tokens per owner, etc.
#[macro_export]
macro_rules! impl_multi_token_enumeration {
    ($contract: ident, $token: ident) => {
        use $crate::multi_token::enumeration::MultiTokenEnumeration;

        #[near_bindgen]
        impl MultiTokenEnumeration for $contract {
            fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
                self.$token.mt_tokens(from_index, limit)
            }

            fn mt_tokens_for_owner(
                &self,
                account_id: AccountId,
                from_index: Option<U128>,
                limit: Option<u64>,
            ) -> Vec<Token> {
                self.$token.mt_tokens_for_owner(account_id, from_index, limit)
            }
        }
    };
}

/// Ensures that when multi token storage grows by collections adding entries,
/// the storage is be paid by the caller. This ensures that storage cannot grow to a point
/// that the FT contract runs out of Ⓝ.
/// Takes name of the Contract struct, the inner field for the token and optional method name to
/// call when the account was closed.
#[macro_export]
macro_rules! impl_multi_token_storage {
    ($contract: ident, $token: ident $(, $on_account_closed_fn:ident)?) => {
        use $crate::storage_management::{
            StorageManagement, StorageBalance, StorageBalanceBounds
        };

        #[near_bindgen]
        impl StorageManagement for $contract {
            #[payable]
            fn storage_deposit(
                &mut self,
                account_id: Option<AccountId>,
                registration_only: Option<bool>,
            ) -> StorageBalance {
                self.$token.storage_deposit(account_id, registration_only)
            }

            #[payable]
            fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
                self.$token.storage_withdraw(amount)
            }

            #[payable]
            fn storage_unregister(&mut self, force: Option<bool>) -> bool {
                #[allow(unused_variables)]
                if let Some((account_id, balance)) = self.$token.internal_storage_unregister(force) {
                    $(self.$on_account_closed_fn(account_id, balance);)?
                    true
                } else {
                    false
                }
            }

            fn storage_balance_bounds(&self) -> StorageBalanceBounds {
                self.$token.storage_balance_bounds()
            }

            fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
                self.$token.storage_balance_of(account_id)
            }
        }
    };
}
