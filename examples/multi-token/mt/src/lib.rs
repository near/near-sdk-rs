/*!
Multi Token implementation with JSON serialization (NEP-245).
NOTES:
  - This implementation supports fungible, semi-fungible, and non-fungible tokens.
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::multi_token::approval::MultiTokenApproval;
use near_contract_standards::multi_token::core::{MultiTokenCore, MultiTokenResolver};
use near_contract_standards::multi_token::enumeration::MultiTokenEnumeration;
use near_contract_standards::multi_token::metadata::{
    MTContractMetadata, MTTokenMetadata, MultiTokenMetadataProvider, MT_METADATA_SPEC,
};
use near_contract_standards::multi_token::{MultiToken, Token, TokenId};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    env, near, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    tokens: MultiToken,
    metadata: LazyOption<MTContractMetadata>,
}

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    MultiToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near]
impl Contract {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        Self::new(
            owner_id,
            MTContractMetadata {
                spec: MT_METADATA_SPEC.to_string(),
                name: "Example NEAR Multi Token".to_string(),
            },
        )
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: MTContractMetadata) -> Self {
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

    /// Mint new tokens.
    ///
    /// Only the contract owner can call this method.
    /// Requires deposit to cover storage costs.
    #[payable]
    pub fn mt_mint(
        &mut self,
        token_id: TokenId,
        token_owner_id: AccountId,
        amount: U128,
        token_metadata: Option<MTTokenMetadata>,
    ) -> Token {
        assert_eq!(env::predecessor_account_id(), self.tokens.owner_id, "Unauthorized");
        self.tokens.internal_mint(token_id, token_owner_id, amount.0, token_metadata)
    }

    /// Burn tokens.
    ///
    /// Only the token holder can burn their own tokens.
    #[payable]
    pub fn mt_burn(&mut self, token_id: TokenId, amount: U128, memo: Option<String>) {
        near_sdk::assert_one_yocto();
        let account_id = env::predecessor_account_id();
        self.tokens.internal_burn(&token_id, &account_id, amount.0, memo);
    }
}

#[near]
impl MultiTokenCore for Contract {
    #[payable]
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) {
        self.tokens.mt_transfer(receiver_id, token_id, amount, approval, memo);
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
        self.tokens.mt_batch_transfer(receiver_id, token_ids, amounts, approvals, memo);
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
    ) -> PromiseOrValue<Vec<U128>> {
        self.tokens.mt_transfer_call(receiver_id, token_id, amount, approval, memo, msg)
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
        self.tokens.mt_batch_transfer_call(receiver_id, token_ids, amounts, approvals, memo, msg)
    }

    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>> {
        self.tokens.mt_token(token_ids)
    }

    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128 {
        self.tokens.mt_balance_of(account_id, token_id)
    }

    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128> {
        self.tokens.mt_batch_balance_of(account_id, token_ids)
    }

    fn mt_supply(&self, token_id: TokenId) -> Option<U128> {
        self.tokens.mt_supply(token_id)
    }

    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>> {
        self.tokens.mt_batch_supply(token_ids)
    }
}

#[near]
impl MultiTokenResolver for Contract {
    #[private]
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<(AccountId, u64, u128)>>>>,
    ) -> Vec<U128> {
        self.tokens.mt_resolve_transfer(
            previous_owner_ids,
            receiver_id,
            token_ids,
            amounts,
            approvals,
        )
    }
}

#[near]
impl MultiTokenApproval for Contract {
    #[payable]
    fn mt_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        self.tokens.mt_approve(token_ids, amounts, account_id, msg)
    }

    #[payable]
    fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId) {
        self.tokens.mt_revoke(token_ids, account_id);
    }

    #[payable]
    fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>) {
        self.tokens.mt_revoke_all(token_ids);
    }

    fn mt_is_approved(
        &self,
        token_ids: Vec<TokenId>,
        approved_account_id: AccountId,
        amounts: Vec<U128>,
        approval_ids: Option<Vec<u64>>,
    ) -> bool {
        self.tokens.mt_is_approved(token_ids, approved_account_id, amounts, approval_ids)
    }
}

#[near]
impl MultiTokenEnumeration for Contract {
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token> {
        self.tokens.mt_tokens(from_index, limit)
    }

    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<Token> {
        self.tokens.mt_tokens_for_owner(account_id, from_index, limit)
    }
}

#[near]
impl MultiTokenMetadataProvider for Contract {
    fn mt_metadata_contract(&self) -> MTContractMetadata {
        self.metadata.get().unwrap()
    }

    fn mt_metadata_token_all(
        &self,
        token_ids: Vec<String>,
    ) -> Vec<near_contract_standards::multi_token::metadata::MTTokenMetadataAll> {
        // For this example, we return empty metadata for tokens without stored metadata
        token_ids
            .into_iter()
            .map(|_token_id| near_contract_standards::multi_token::metadata::MTTokenMetadataAll {
                base: near_contract_standards::multi_token::metadata::MTBaseTokenMetadata::default(
                ),
                token: MTTokenMetadata::default(),
            })
            .collect()
    }

    fn mt_metadata_token_by_token_id(&self, token_ids: Vec<String>) -> Vec<MTTokenMetadata> {
        token_ids
            .into_iter()
            .map(|token_id| {
                self.tokens
                    .token_metadata_by_id
                    .as_ref()
                    .and_then(|m| m.get(&token_id))
                    .unwrap_or_default()
            })
            .collect()
    }

    fn mt_metadata_base_by_token_id(
        &self,
        _token_ids: Vec<String>,
    ) -> Vec<near_contract_standards::multi_token::metadata::MTBaseTokenMetadata> {
        // Base metadata is not stored in this example
        vec![]
    }

    fn mt_metadata_base_by_metadata_id(
        &self,
        _base_metadata_ids: Vec<String>,
    ) -> Vec<near_contract_standards::multi_token::metadata::MTBaseTokenMetadata> {
        // Base metadata is not stored in this example
        vec![]
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, NearToken};

    use super::*;

    const MINT_STORAGE_COST: NearToken =
        NearToken::from_yoctonear(6_000_000_000_000_000_000_000u128);

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn sample_token_metadata() -> MTTokenMetadata {
        MTTokenMetadata {
            title: Some("Silver Sword".into()),
            description: Some("A legendary silver sword".into()),
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
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into());
        testing_env!(context.is_view(true).build());
        let tokens = contract.mt_token(vec!["1".to_string()]);
        assert_eq!(tokens[0], None);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_mint() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());

        let token_id = "sword-1".to_string();
        let token = contract.mt_mint(
            token_id.clone(),
            accounts(0),
            U128(100),
            Some(sample_token_metadata()),
        );
        assert_eq!(token.token_id, token_id);
        assert_eq!(token.owner_id, Some(accounts(0)));
        assert_eq!(token.metadata.unwrap(), sample_token_metadata());

        // Check balance
        assert_eq!(contract.mt_balance_of(accounts(0), token_id.clone()).0, 100);

        // Check supply
        assert_eq!(contract.mt_supply(token_id).unwrap().0, 100);
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "sword-1".to_string();
        contract.mt_mint(token_id.clone(), accounts(0), U128(100), Some(sample_token_metadata()));

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_transfer(accounts(1), token_id.clone(), U128(30), None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build());

        // Check balances
        assert_eq!(contract.mt_balance_of(accounts(0), token_id.clone()).0, 70);
        assert_eq!(contract.mt_balance_of(accounts(1), token_id.clone()).0, 30);

        // Supply should remain unchanged
        assert_eq!(contract.mt_supply(token_id).unwrap().0, 100);
    }

    #[test]
    fn test_burn() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "sword-1".to_string();
        contract.mt_mint(token_id.clone(), accounts(0), U128(100), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_burn(token_id.clone(), U128(40), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build());

        // Check balance reduced
        assert_eq!(contract.mt_balance_of(accounts(0), token_id.clone()).0, 60);

        // Supply should be reduced too
        assert_eq!(contract.mt_supply(token_id).unwrap().0, 60);
    }

    #[test]
    fn test_batch_transfer() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());

        // Mint two different token types
        contract.mt_mint("sword-1".to_string(), accounts(0), U128(100), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_mint("potion-1".to_string(), accounts(0), U128(50), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build());

        // Batch transfer both
        contract.mt_batch_transfer(
            accounts(1),
            vec!["sword-1".to_string(), "potion-1".to_string()],
            vec![U128(10), U128(5)],
            None,
            None,
        );

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build());

        // Check balances
        assert_eq!(contract.mt_balance_of(accounts(0), "sword-1".to_string()).0, 90);
        assert_eq!(contract.mt_balance_of(accounts(1), "sword-1".to_string()).0, 10);
        assert_eq!(contract.mt_balance_of(accounts(0), "potion-1".to_string()).0, 45);
        assert_eq!(contract.mt_balance_of(accounts(1), "potion-1".to_string()).0, 5);
    }

    #[test]
    fn test_approve() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "sword-1".to_string();
        contract.mt_mint(token_id.clone(), accounts(0), U128(100), None);

        // Approve bob to spend 50 tokens
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(150_000_000_000_000_000_000u128))
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_approve(vec![token_id.clone()], vec![U128(50)], accounts(1), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build());

        assert!(contract.mt_is_approved(vec![token_id.clone()], accounts(1), vec![U128(50)], None));

        // Not approved for more than 50
        assert!(!contract.mt_is_approved(vec![token_id], accounts(1), vec![U128(51)], None));
    }

    #[test]
    fn test_revoke() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "sword-1".to_string();
        contract.mt_mint(token_id.clone(), accounts(0), U128(100), None);

        // Approve bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(150_000_000_000_000_000_000u128))
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_approve(vec![token_id.clone()], vec![U128(50)], accounts(1), None);

        // Revoke bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_revoke(vec![token_id.clone()], accounts(1));

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build());

        assert!(!contract.mt_is_approved(vec![token_id], accounts(1), vec![U128(1)], None));
    }

    #[test]
    fn test_enumeration() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_mint("sword-1".to_string(), accounts(0), U128(100), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        contract.mt_mint("potion-1".to_string(), accounts(1), U128(50), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build());

        // Get all tokens
        let tokens = contract.mt_tokens(None, None);
        assert_eq!(tokens.len(), 2);

        // Get tokens for owner
        let owner_tokens = contract.mt_tokens_for_owner(accounts(0), None, None);
        assert_eq!(owner_tokens.len(), 1);
        assert_eq!(owner_tokens[0].token_id, "sword-1".to_string());
    }
}
