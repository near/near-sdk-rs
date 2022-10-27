use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption};
use near_sdk::json_types::U128;
use near_sdk::{Promise};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, PromiseOrValue,
};
use near_contract_standards::multi_token::metadata::MT_METADATA_SPEC;
use near_contract_standards::multi_token::token::{Token, TokenId};
use near_contract_standards::multi_token::{
    core::MultiToken,
    metadata::{MtContractMetadata, TokenMetadata},
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ExampleMTContract {
    tokens: MultiToken,
    metadata: LazyOption<MtContractMetadata>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    MultiToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl ExampleMTContract {
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        let metadata = MtContractMetadata {
            spec: MT_METADATA_SPEC.to_string(),
            name: "Example NEAR multi token".to_string(),
            symbol: "EXAMPLE".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        };

        Self::new(owner_id, metadata)
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: MtContractMetadata) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();

        Self {
            tokens: MultiToken::new(
                StorageKey::MultiToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    #[payable]
    pub fn mt_mint(
        &mut self,
        token_owner_id: AccountId,
        token_metadata: TokenMetadata,
        supply: U128,
    ) -> Token {
        // Only the owner of the MT contract can perform this operation
        assert_eq!(
            env::predecessor_account_id(),
            self.tokens.owner_id,
            "Unauthorized: {} != {}",
            env::predecessor_account_id(),
            self.tokens.owner_id
        );
        self.tokens.internal_mint(token_owner_id, Some(supply.into()), Some(token_metadata), None)
    }
}

near_contract_standards::impl_multi_token_core!(ExampleMTContract, tokens);
near_contract_standards::impl_multi_token_approval!(ExampleMTContract, tokens);
near_contract_standards::impl_multi_token_enumeration!(ExampleMTContract, tokens);
near_contract_standards::impl_multi_token_storage!(ExampleMTContract, tokens);

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn create_token_md(title: String, description: String) -> TokenMetadata {
        TokenMetadata {
            title: Some(title),
            description: Some(description),
            media: None,
            media_hash: None,
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        }
    }

    #[test]
    fn test_transfer() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);

        // Initial balances are what we expect.
        assert_eq!(
            contract.mt_balance_of(accounts(0), token.token_id.clone()),
            U128(1000),
            "Wrong balance"
        );
        assert_eq!(
            contract.mt_balance_of(accounts(1), token.token_id.clone()),
            U128(0),
            "Wrong balance"
        );

        // Transfer some tokens
        testing_env!(context.attached_deposit(1).build());
        // emulate storage_deposit
        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &contract.tokens.storage_balance_bounds().min.into(),
        );

        contract.mt_transfer(accounts(1), token.token_id.clone(), 4.into(), None, None);

        // Transfer should have succeeded.
        assert_eq!(
            contract.mt_balance_of(accounts(0), token.token_id.clone()).0,
            996,
            "Wrong balance"
        );
        assert_eq!(
            contract.mt_balance_of(accounts(1), token.token_id.clone()).0,
            4,
            "Wrong balance"
        );

        // Transfer some of the tokens back to original owner.
        set_caller(&mut context, 1);

        contract.mt_transfer(accounts(0), token.token_id.clone(), 3.into(), None, None);

        assert_eq!(
            contract.mt_balance_of(accounts(0), token.token_id.clone()).0,
            999,
            "Wrong balance"
        );
        assert_eq!(
            contract.mt_balance_of(accounts(1), token.token_id.clone()).0,
            1,
            "Wrong balance"
        );
    }

    #[test]
    #[should_panic(expected = "Transferred amounts must be greater than 0")]
    fn test_transfer_amount_must_be_positive() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);
        testing_env!(context.attached_deposit(1).build());
        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &contract.tokens.storage_balance_bounds().min.into(),
        );
        contract.mt_transfer(accounts(1), token.token_id.clone(), U128(0), None, None)
    }

    #[test]
    #[should_panic(expected = "No approvals for token 1")]
    fn test_transfer_no_approvals_for_account() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);
        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &contract.tokens.storage_balance_bounds().min.into(),
        );

        testing_env!(context.attached_deposit(1).build());
        contract.mt_transfer(
            accounts(1),
            token.token_id.clone(),
            U128(1),
            Some((accounts(0), 1)),
            None
        )
    }

    #[test]
    #[should_panic(expected = "The account doesn't have enough balance")]
    fn test_sender_account_must_have_sufficient_balance() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);
        testing_env!(context.attached_deposit(1).build());
        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &contract.tokens.storage_balance_bounds().min.into(),
        );
        // account(0) has only 2000 of token.
        contract.mt_transfer(accounts(1), token.token_id.clone(), U128(3000), None, None)
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn test_transfers_require_one_yocto() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);
        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &contract.tokens.storage_balance_bounds().min.into(),
        );
        contract.mt_transfer(accounts(1), token.token_id.clone(), U128(1000), None, None)
    }

    #[test]
    #[should_panic(expected = "The account is not registered")]
    fn test_receiver_must_be_registered() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);
        testing_env!(context.attached_deposit(1).build());
        contract.mt_transfer(accounts(2), token.token_id.clone(), U128(100), None, None)
    }

    #[test]
    #[should_panic(expected = "Sender and receiver must differ")]
    fn test_cannot_transfer_to_self() {
        let mut context = VMContextBuilder::new();
        set_caller(&mut context, 0);
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        let (token, _) = init_tokens(&mut contract);
        testing_env!(context.attached_deposit(1).build());
        contract.mt_transfer(accounts(0), token.token_id.clone(), U128(100), None, None)
    }

    #[test]
    fn test_batch_transfer() {
        let mut context = VMContextBuilder::new();
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        set_caller(&mut context, 0);

        let (quote_token, base_token) = init_tokens(&mut contract);

        testing_env!(context.attached_deposit(1).build());

        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &(contract.tokens.storage_balance_bounds().min.0 * 2),
        );
        // Perform the transfers
        contract.mt_batch_transfer(
            accounts(1),
            vec![quote_token.token_id.clone(), base_token.token_id.clone()],
            vec![U128(4), U128(600)],
            None,
            None,
        );

        assert_eq!(
            contract.mt_balance_of(accounts(0), quote_token.token_id.clone()).0,
            996,
            "Wrong balance"
        );
        assert_eq!(
            contract.mt_balance_of(accounts(1), quote_token.token_id.clone()).0,
            4,
            "Wrong balance"
        );

        assert_eq!(
            contract.mt_balance_of(accounts(0), base_token.token_id.clone()).0,
            1400,
            "Wrong balance"
        );
        assert_eq!(
            contract.mt_balance_of(accounts(1), base_token.token_id.clone()).0,
            600,
            "Wrong balance"
        );
    }

    #[test]
    #[should_panic(expected = "The account doesn't have enough balance")]
    fn test_batch_transfer_all_balances_must_be_sufficient() {
        let mut context = VMContextBuilder::new();
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        set_caller(&mut context, 0);

        let (quote_token, base_token) = init_tokens(&mut contract);

        testing_env!(context.attached_deposit(1).build());
        contract.tokens.accounts_storage.insert(
            &accounts(1),
            &(contract.tokens.storage_balance_bounds().min.0 * 2),
        );
        contract.mt_batch_transfer(
            accounts(1),
            vec![quote_token.token_id.clone(), base_token.token_id.clone()],
            vec![U128(4), U128(6000)],
            None,
            None,
        );
    }

    #[test]
    fn test_simple_approvals() {
        let mut context = VMContextBuilder::new();
        let mut contract = ExampleMTContract::new_default_meta(accounts(0));
        set_caller(&mut context, 0);

        let (quote_token, base_token) = init_tokens(&mut contract);

        let owner_id = accounts(0);

        // Initially, Account 1 is not approved.
        testing_env!(context.attached_deposit(1).build());
        assert!(!contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone()],
            accounts(1),
            vec![U128(20)],
            None,
        ));

        // Create approval for account 1 to transfer 20 of quote token from account 0.
        testing_env!(context.attached_deposit(150000000000000000000).build());
        contract.mt_approve(
            vec![quote_token.token_id.clone()],
            vec![U128(20)],
            accounts(1),
            None,
        );

        // Account 1 is approved for 20 tokens.
        testing_env!(context.attached_deposit(1).build());
        assert!(contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone()],
            accounts(1),
            vec![U128(20)],
            None,
        ));

        // Account 1 is NOT approved for more than 20 tokens.
        testing_env!(context.attached_deposit(1).build());
        assert!(!contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone()],
            accounts(1),
            vec![U128(21)],
            None,
        ));

        // Account 1 is NOT approved for the other token.
        testing_env!(context.attached_deposit(1).build());
        assert!(!contract.mt_is_approved(
            owner_id.clone(),
            vec![base_token.token_id.clone()],
            accounts(1),
            vec![U128(20)],
            None,
        ));

        // Revoke the approval
        contract.mt_revoke(
            vec![quote_token.token_id.clone()],
            accounts(1),
        );
        assert!(!contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone()],
            accounts(1),
            vec![U128(20)],
            None,
        ));

        // Create 2 approvals for 2 tokens in one call.
        testing_env!(context.attached_deposit(2 * 150000000000000000000).build());
        contract.mt_approve(
            vec![quote_token.token_id.clone(), base_token.token_id.clone()],
            vec![U128(10), U128(500)],
            accounts(1),
            None,
        );
        assert!(contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone(), base_token.token_id.clone()],
            accounts(1),
            vec![U128(10), U128(500)],
            None,
        ));

        // Approve a different account
        contract.mt_approve(
            vec![quote_token.token_id.clone()],
            vec![U128(30)],
            accounts(2),
            None,
        );

        // Revoke all approvals for the quote token
        testing_env!(context.attached_deposit(1).build());
        contract.mt_revoke_all(
            vec![quote_token.token_id.clone()],
        );

        // Neither account is still approved
        assert!(!contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone(), base_token.token_id.clone()],
            accounts(1),
            vec![U128(10), U128(500)],
            None,
        ));
        assert!(!contract.mt_is_approved(
            owner_id.clone(),
            vec![quote_token.token_id.clone()],
            accounts(2),
            vec![U128(30)],
            None,
        ));
    }

    fn init_tokens(contract: &mut ExampleMTContract) -> (Token, Token) {
        let quote_token_md = create_token_md("PYC".into(), "Python token".into());
        let base_token_md = create_token_md("ABC".into(), "Alphabet token".into());

        // emulate storage_deposit
        contract.tokens.accounts_storage.insert(
            &accounts(0),
            &(contract.tokens.storage_balance_bounds().min.0 * 2),
        );

        let quote_token = contract.mt_mint(accounts(0), quote_token_md.clone(), U128(1000));
        let base_token = contract.mt_mint(accounts(0), base_token_md.clone(), U128(2000));

        (quote_token, base_token)
    }

    fn set_caller(context: &mut VMContextBuilder, account_id: usize) {
        testing_env!(context
            .signer_account_id(accounts(account_id))
            .predecessor_account_id(accounts(account_id))
            .build())
    }
}
