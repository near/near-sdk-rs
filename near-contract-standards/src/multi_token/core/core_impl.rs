//! Core implementation of the Multi Token standard (NEP-245).
//!
//! This module provides the `MultiToken` struct which handles all state management
//! for multi-token contracts. It implements the core transfer functionality and
//! can be extended with optional features like metadata, enumeration, and approvals.

use std::collections::HashMap;

use near_sdk::{
    assert_one_yocto,
    borsh::BorshSerialize,
    collections::{LookupMap, TreeMap, UnorderedSet},
    env,
    json_types::U128,
    log, near, require, AccountId, BorshStorageKey, Gas, IntoStorageKey, PromiseOrValue,
    PromiseResult, StorageUsage,
};

use crate::multi_token::{
    core::{receiver::ext_mt_receiver, resolver::ext_mt_resolver, MultiTokenCore},
    events::{MtBurn, MtMint, MtTransfer},
    metadata::MTTokenMetadata,
    token::{Approval, ClearedApproval, Token, TokenId},
    utils::refund_deposit_to_account,
};

use super::resolver::MultiTokenResolver;

const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_tgas(5);
const GAS_FOR_MT_TRANSFER_CALL: Gas = Gas::from_tgas(30);

const ERR_TOTAL_SUPPLY_OVERFLOW: &str = "Total supply overflow";
const ERR_BALANCE_OVERFLOW: &str = "Balance overflow";

/// Internal storage key for nested collections
#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    TokensPerOwner { account_hash: Vec<u8> },
    BalancesPerToken { token_id_hash: Vec<u8> },
}

/// Implementation of the multi-token standard.
///
/// Allows including NEP-245 compatible tokens in any contract.
/// There are several traits that any contract may implement:
///
/// - `MultiTokenCore` -- interface with mt_transfer methods. MultiToken provides methods for it.
/// - `MultiTokenApproval` -- interface with mt_approve methods. MultiToken provides methods for it.
/// - `MultiTokenEnumeration` -- interface for getting lists of tokens. MultiToken provides methods for it.
/// - `MultiTokenMetadataProvider` -- return metadata for the token in NEP-245, up to contract to implement.
///
/// For example usage, see examples/multi-token/src/lib.rs.
#[near]
pub struct MultiToken {
    /// Owner of the contract (can mint new tokens)
    pub owner_id: AccountId,

    /// The storage size in bytes for each new token
    pub extra_storage_in_bytes_per_token: StorageUsage,

    /// TokenId -> creator/owner for NFT-like tokens (where there's a single owner)
    /// For fungible-style tokens, this stores the original creator
    pub owner_by_id: TreeMap<TokenId, AccountId>,

    /// TokenId -> total supply
    pub total_supply: LookupMap<TokenId, u128>,

    /// TokenId -> (AccountId -> balance)
    /// Nested LookupMap for efficient per-token balance lookups
    pub balances: LookupMap<TokenId, LookupMap<AccountId, u128>>,

    /// Required by metadata extension: TokenId -> TokenMetadata
    pub token_metadata_by_id: Option<LookupMap<TokenId, MTTokenMetadata>>,

    /// Required by enumeration extension: AccountId -> set of TokenIds owned
    pub tokens_per_owner: Option<LookupMap<AccountId, UnorderedSet<TokenId>>>,

    /// Required by approval extension: TokenId -> (OwnerAccountId -> (ApprovedAccountId -> Approval))
    #[allow(clippy::type_complexity)]
    pub approvals_by_id:
        Option<LookupMap<TokenId, HashMap<AccountId, HashMap<AccountId, Approval>>>>,

    /// Next approval ID for each token
    pub next_approval_id_by_id: Option<LookupMap<TokenId, u64>>,
}

impl MultiToken {
    /// Create a new MultiToken collection.
    ///
    /// # Arguments
    ///
    /// * `owner_by_id_prefix` - Storage prefix for the owner_by_id collection
    /// * `owner_id` - Account ID of the contract owner (can mint tokens)
    /// * `token_metadata_prefix` - Optional storage prefix for metadata extension
    /// * `enumeration_prefix` - Optional storage prefix for enumeration extension
    /// * `approval_prefix` - Optional storage prefix for approval extension
    pub fn new<O, T, E, A>(
        owner_by_id_prefix: O,
        owner_id: AccountId,
        token_metadata_prefix: Option<T>,
        enumeration_prefix: Option<E>,
        approval_prefix: Option<A>,
    ) -> Self
    where
        O: IntoStorageKey,
        T: IntoStorageKey,
        E: IntoStorageKey,
        A: IntoStorageKey,
    {
        let (approvals_by_id, next_approval_id_by_id) = if let Some(prefix) = approval_prefix {
            let prefix: Vec<u8> = prefix.into_storage_key();
            (
                Some(LookupMap::new(prefix.clone())),
                Some(LookupMap::new([prefix, b"n".to_vec()].concat())),
            )
        } else {
            (None, None)
        };

        let owner_prefix: Vec<u8> = owner_by_id_prefix.into_storage_key();

        let mut this = Self {
            owner_id,
            extra_storage_in_bytes_per_token: 0,
            owner_by_id: TreeMap::new(owner_prefix.clone()),
            total_supply: LookupMap::new([owner_prefix.clone(), b"s".to_vec()].concat()),
            balances: LookupMap::new([owner_prefix, b"b".to_vec()].concat()),
            token_metadata_by_id: token_metadata_prefix.map(LookupMap::new),
            tokens_per_owner: enumeration_prefix.map(LookupMap::new),
            approvals_by_id,
            next_approval_id_by_id,
        };
        this.measure_min_token_storage_cost();
        this
    }

    /// Measure the minimum storage cost for a token.
    fn measure_min_token_storage_cost(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let tmp_token_id = "a".repeat(64);
        let tmp_account_id: AccountId = "a".repeat(64).parse().unwrap();

        // Add dummy data to measure storage
        self.owner_by_id.insert(&tmp_token_id, &tmp_account_id);
        self.total_supply.insert(&tmp_token_id, &0u128);

        // Create nested balance map
        let mut balance_map = LookupMap::new(StorageKey::BalancesPerToken {
            token_id_hash: env::sha256(tmp_token_id.as_bytes()),
        });
        balance_map.insert(&tmp_account_id, &0u128);
        self.balances.insert(&tmp_token_id, &balance_map);

        if let Some(token_metadata_by_id) = &mut self.token_metadata_by_id {
            token_metadata_by_id.insert(&tmp_token_id, &MTTokenMetadata::default());
        }

        if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
            let mut set = UnorderedSet::new(StorageKey::TokensPerOwner {
                account_hash: env::sha256(tmp_account_id.as_bytes()),
            });
            set.insert(&tmp_token_id);
            tokens_per_owner.insert(&tmp_account_id, &set);
        }

        if let Some(approvals_by_id) = &mut self.approvals_by_id {
            let mut approvals = HashMap::new();
            let mut inner = HashMap::new();
            inner.insert(tmp_account_id.clone(), Approval { approval_id: 0, amount: 0 });
            approvals.insert(tmp_account_id.clone(), inner);
            approvals_by_id.insert(&tmp_token_id, &approvals);
        }

        if let Some(next_approval_id_by_id) = &mut self.next_approval_id_by_id {
            next_approval_id_by_id.insert(&tmp_token_id, &0u64);
        }

        self.extra_storage_in_bytes_per_token = env::storage_usage() - initial_storage_usage;

        // Clean up
        if let Some(next_approval_id_by_id) = &mut self.next_approval_id_by_id {
            next_approval_id_by_id.remove(&tmp_token_id);
        }
        if let Some(approvals_by_id) = &mut self.approvals_by_id {
            approvals_by_id.remove(&tmp_token_id);
        }
        if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
            let mut set = tokens_per_owner.remove(&tmp_account_id).unwrap();
            set.remove(&tmp_token_id);
        }
        if let Some(token_metadata_by_id) = &mut self.token_metadata_by_id {
            token_metadata_by_id.remove(&tmp_token_id);
        }
        self.balances.remove(&tmp_token_id);
        self.total_supply.remove(&tmp_token_id);
        self.owner_by_id.remove(&tmp_token_id);
    }

    /// Check if a token exists.
    pub fn token_exists(&self, token_id: &TokenId) -> bool {
        self.total_supply.contains_key(token_id)
    }

    /// Get the balance of an account for a specific token.
    /// Returns 0 if the account or token doesn't exist.
    pub fn internal_balance_of(&self, account_id: &AccountId, token_id: &TokenId) -> u128 {
        self.balances.get(token_id).and_then(|balances| balances.get(account_id)).unwrap_or(0)
    }

    /// Get the balance of an account for a specific token.
    /// Panics if the token doesn't exist or account has no balance.
    pub fn internal_unwrap_balance_of(&self, account_id: &AccountId, token_id: &TokenId) -> u128 {
        self.balances.get(token_id).and_then(|balances| balances.get(account_id)).unwrap_or_else(
            || {
                env::panic_str(&format!(
                    "Account {} has no balance for token {}",
                    account_id, token_id
                ))
            },
        )
    }

    /// Get the total supply of a token.
    pub fn internal_supply(&self, token_id: &TokenId) -> Option<u128> {
        self.total_supply.get(token_id)
    }

    /// Internal function to update balance for an account.
    /// Creates the nested map if it doesn't exist.
    fn internal_set_balance(&mut self, token_id: &TokenId, account_id: &AccountId, balance: u128) {
        let mut balances = self.balances.get(token_id).unwrap_or_else(|| {
            LookupMap::new(StorageKey::BalancesPerToken {
                token_id_hash: env::sha256(token_id.as_bytes()),
            })
        });

        if balance == 0 {
            balances.remove(account_id);
        } else {
            balances.insert(account_id, &balance);
        }

        self.balances.insert(token_id, &balances);
    }

    /// Mint new tokens.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Unique identifier for the token type
    /// * `token_owner_id` - Account to receive the minted tokens
    /// * `amount` - Number of tokens to mint
    /// * `token_metadata` - Optional metadata for the token
    ///
    /// # Returns
    ///
    /// The Token struct representing the minted token.
    pub fn internal_mint(
        &mut self,
        token_id: TokenId,
        token_owner_id: AccountId,
        amount: u128,
        token_metadata: Option<MTTokenMetadata>,
    ) -> Token {
        let initial_storage_usage = env::storage_usage();

        // Check if this is a new token type
        let is_new_token = !self.token_exists(&token_id);

        if is_new_token {
            // New token type - set initial owner and supply
            self.owner_by_id.insert(&token_id, &token_owner_id);
            self.total_supply.insert(&token_id, &amount);

            // Store metadata if provided and extension is enabled
            if let Some(metadata) = &token_metadata {
                if let Some(token_metadata_by_id) = &mut self.token_metadata_by_id {
                    token_metadata_by_id.insert(&token_id, metadata);
                }
            }
        } else {
            // Existing token - add to supply
            let current_supply = self.total_supply.get(&token_id).unwrap_or(0);
            let new_supply = current_supply
                .checked_add(amount)
                .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW));
            self.total_supply.insert(&token_id, &new_supply);
        }

        // Add balance to the recipient
        let current_balance = self.internal_balance_of(&token_owner_id, &token_id);
        let new_balance = current_balance
            .checked_add(amount)
            .unwrap_or_else(|| env::panic_str(ERR_BALANCE_OVERFLOW));
        self.internal_set_balance(&token_id, &token_owner_id, new_balance);

        // Update enumeration if enabled
        if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
            let mut token_set = tokens_per_owner.get(&token_owner_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::TokensPerOwner {
                    account_hash: env::sha256(token_owner_id.as_bytes()),
                })
            });
            token_set.insert(&token_id);
            tokens_per_owner.insert(&token_owner_id, &token_set);
        }

        // Refund unused deposit
        let storage_used = env::storage_usage() - initial_storage_usage;
        refund_deposit_to_account(storage_used, env::predecessor_account_id());

        // Emit mint event
        let token_ids: Vec<&str> = vec![token_id.as_str()];
        let amounts: Vec<U128> = vec![U128(amount)];
        MtMint {
            owner_id: token_owner_id.as_ref(),
            token_ids: &token_ids,
            amounts: &amounts,
            memo: None,
        }
        .emit();

        // Build and return Token
        Token {
            token_id,
            owner_id: Some(token_owner_id),
            metadata: token_metadata,
            approved_account_ids: None,
        }
    }

    /// Burn tokens.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token type to burn
    /// * `account_id` - Account to burn from
    /// * `amount` - Number of tokens to burn
    /// * `memo` - Optional memo for the burn event
    pub fn internal_burn(
        &mut self,
        token_id: &TokenId,
        account_id: &AccountId,
        amount: u128,
        memo: Option<String>,
    ) {
        require!(self.token_exists(token_id), "Token does not exist");

        // Check and update balance
        let current_balance = self.internal_balance_of(account_id, token_id);
        require!(
            current_balance >= amount,
            format!("Not enough balance. Required: {}, Available: {}", amount, current_balance)
        );

        let new_balance = current_balance - amount;
        self.internal_set_balance(token_id, account_id, new_balance);

        // Update total supply
        let current_supply = self.total_supply.get(token_id).unwrap_or(0);
        let new_supply = current_supply.saturating_sub(amount);
        self.total_supply.insert(token_id, &new_supply);

        // Update enumeration if balance is now 0
        if new_balance == 0 {
            if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
                if let Some(mut token_set) = tokens_per_owner.get(account_id) {
                    token_set.remove(token_id);
                    if token_set.is_empty() {
                        tokens_per_owner.remove(account_id);
                    } else {
                        tokens_per_owner.insert(account_id, &token_set);
                    }
                }
            }
        }

        // Emit burn event
        let token_ids: Vec<&str> = vec![token_id.as_str()];
        let amounts: Vec<U128> = vec![U128(amount)];
        MtBurn {
            owner_id: account_id.as_ref(),
            token_ids: &token_ids,
            amounts: &amounts,
            authorized_id: None,
            memo: memo.as_deref(),
        }
        .emit();
    }

    /// Internal transfer implementation.
    ///
    /// Returns the previous owner's approvals (for potential refund).
    pub fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> (AccountId, Option<HashMap<AccountId, Approval>>) {
        require!(self.token_exists(token_id), "Token does not exist");
        require!(sender_id != receiver_id, "Cannot transfer to self");
        require!(amount > 0, "Amount must be positive");

        let predecessor_id = env::predecessor_account_id();

        // Check authorization
        let mut cleared_approval: Option<HashMap<AccountId, Approval>> = None;
        if &predecessor_id != sender_id {
            // Check if predecessor is approved
            if let Some(approvals_by_id) = &mut self.approvals_by_id {
                if let Some(mut token_approvals) = approvals_by_id.get(token_id) {
                    if let Some(owner_approvals) = token_approvals.get_mut(sender_id) {
                        if let Some(approval) = owner_approvals.get(&predecessor_id) {
                            // Verify approval_id if provided
                            if let Some(expected_approval_id) = approval_id {
                                require!(
                                    approval.approval_id == expected_approval_id,
                                    "Approval ID mismatch"
                                );
                            }
                            require!(approval.amount >= amount, "Approved amount insufficient");

                            // Update or remove approval
                            if approval.amount == amount {
                                cleared_approval = Some(owner_approvals.clone());
                                owner_approvals.remove(&predecessor_id);
                            } else {
                                owner_approvals.insert(
                                    predecessor_id.clone(),
                                    Approval {
                                        approval_id: approval.approval_id,
                                        amount: approval.amount - amount,
                                    },
                                );
                            }
                            approvals_by_id.insert(token_id, &token_approvals);
                        } else {
                            env::panic_str("Not approved");
                        }
                    } else {
                        env::panic_str("Not approved");
                    }
                } else {
                    env::panic_str("Not approved");
                }
            } else {
                env::panic_str("Approval extension not enabled");
            }
        }

        // Update sender balance
        let sender_balance = self.internal_balance_of(sender_id, token_id);
        require!(
            sender_balance >= amount,
            format!(
                "Sender {} does not have enough balance. Required: {}, Available: {}",
                sender_id, amount, sender_balance
            )
        );
        self.internal_set_balance(token_id, sender_id, sender_balance - amount);

        // Update receiver balance
        let receiver_balance = self.internal_balance_of(receiver_id, token_id);
        let new_receiver_balance = receiver_balance
            .checked_add(amount)
            .unwrap_or_else(|| env::panic_str(ERR_BALANCE_OVERFLOW));
        self.internal_set_balance(token_id, receiver_id, new_receiver_balance);

        // Update enumeration
        if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
            // Remove from sender if balance is now 0
            if sender_balance - amount == 0 {
                if let Some(mut sender_tokens) = tokens_per_owner.get(sender_id) {
                    sender_tokens.remove(token_id);
                    if sender_tokens.is_empty() {
                        tokens_per_owner.remove(sender_id);
                    } else {
                        tokens_per_owner.insert(sender_id, &sender_tokens);
                    }
                }
            }

            // Add to receiver
            let mut receiver_tokens = tokens_per_owner.get(receiver_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::TokensPerOwner {
                    account_hash: env::sha256(receiver_id.as_bytes()),
                })
            });
            receiver_tokens.insert(token_id);
            tokens_per_owner.insert(receiver_id, &receiver_tokens);
        }

        // Emit transfer event
        let token_ids: Vec<&str> = vec![token_id.as_str()];
        let amounts: Vec<U128> = vec![U128(amount)];
        MtTransfer {
            old_owner_id: sender_id.as_ref(),
            new_owner_id: receiver_id.as_ref(),
            token_ids: &token_ids,
            amounts: &amounts,
            authorized_id: if &predecessor_id != sender_id {
                Some(predecessor_id.as_ref())
            } else {
                None
            },
            memo: memo.as_deref(),
        }
        .emit();

        (sender_id.clone(), cleared_approval)
    }

    /// Internal transfer and call implementation.
    #[allow(clippy::too_many_arguments)]
    pub fn internal_transfer_call(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let (old_owner, old_approvals) = self.internal_transfer(
            &sender_id,
            &receiver_id,
            &token_id,
            amount.0,
            approval_id,
            memo,
        );

        // Prepare arguments for receiver
        let token_ids = vec![token_id.clone()];
        let amounts = vec![amount];
        let previous_owner_ids = vec![old_owner.clone()];

        // Convert approvals to ClearedApproval format
        let cleared_approvals: Option<Vec<Option<Vec<ClearedApproval>>>> =
            old_approvals.map(|approvals| {
                vec![Some(
                    approvals
                        .into_iter()
                        .map(|(account_id, approval)| {
                            (account_id, approval.approval_id, approval.amount)
                        })
                        .collect(),
                )]
            });

        // Call receiver
        ext_mt_receiver::ext(receiver_id.clone())
            .with_static_gas(
                env::prepaid_gas()
                    .saturating_sub(GAS_FOR_MT_TRANSFER_CALL)
                    .saturating_sub(GAS_FOR_RESOLVE_TRANSFER),
            )
            .mt_on_transfer(
                sender_id.clone(),
                previous_owner_ids.clone(),
                token_ids.clone(),
                amounts.clone(),
                msg,
            )
            .then(
                ext_mt_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .mt_resolve_transfer(
                        previous_owner_ids,
                        receiver_id,
                        token_ids,
                        amounts,
                        cleared_approvals,
                    ),
            )
            .into()
    }
}

impl MultiTokenCore for MultiToken {
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        let sender_id = approval
            .as_ref()
            .map(|(owner, _)| owner.clone())
            .unwrap_or_else(env::predecessor_account_id);
        let approval_id = approval.map(|(_, id)| id);
        self.internal_transfer(&sender_id, &receiver_id, &token_id, amount.0, approval_id, memo);
    }

    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        require!(
            token_ids.len() == amounts.len(),
            "token_ids and amounts must have the same length"
        );
        if let Some(ref approvals) = approvals {
            require!(
                token_ids.len() == approvals.len(),
                "approvals must have the same length as token_ids"
            );
        }

        for (i, (token_id, amount)) in token_ids.iter().zip(amounts.iter()).enumerate() {
            let approval = approvals.as_ref().and_then(|a| a.get(i).cloned().flatten());
            let sender_id = approval
                .as_ref()
                .map(|(owner, _)| owner.clone())
                .unwrap_or_else(env::predecessor_account_id);
            let approval_id = approval.map(|(_, id)| id);
            self.internal_transfer(
                &sender_id,
                &receiver_id,
                token_id,
                amount.0,
                approval_id,
                memo.clone(),
            );
        }
    }

    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        let sender_id = approval
            .as_ref()
            .map(|(owner, _)| owner.clone())
            .unwrap_or_else(env::predecessor_account_id);
        let approval_id = approval.map(|(_, id)| id);
        self.internal_transfer_call(
            sender_id,
            receiver_id,
            token_id,
            amount,
            approval_id,
            memo,
            msg,
        )
    }

    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        require!(
            token_ids.len() == amounts.len(),
            "token_ids and amounts must have the same length"
        );
        if let Some(ref approvals) = approvals {
            require!(
                token_ids.len() == approvals.len(),
                "approvals must have the same length as token_ids"
            );
        }

        // For batch transfer_call, we need to transfer all tokens first,
        // then make a single call to the receiver
        let predecessor_id = env::predecessor_account_id();
        let mut previous_owner_ids = Vec::with_capacity(token_ids.len());
        let mut all_cleared_approvals: Vec<Option<Vec<ClearedApproval>>> =
            Vec::with_capacity(token_ids.len());

        for (i, (token_id, amount)) in token_ids.iter().zip(amounts.iter()).enumerate() {
            let approval = approvals.as_ref().and_then(|a| a.get(i).cloned().flatten());
            let sender_id = approval
                .as_ref()
                .map(|(owner, _)| owner.clone())
                .unwrap_or_else(|| predecessor_id.clone());
            let approval_id = approval.map(|(_, id)| id);

            let (old_owner, old_approvals) = self.internal_transfer(
                &sender_id,
                &receiver_id,
                token_id,
                amount.0,
                approval_id,
                memo.clone(),
            );

            previous_owner_ids.push(old_owner);
            all_cleared_approvals.push(old_approvals.map(|approvals| {
                approvals
                    .into_iter()
                    .map(|(account_id, approval)| {
                        (account_id, approval.approval_id, approval.amount)
                    })
                    .collect()
            }));
        }

        let cleared_approvals = if all_cleared_approvals.iter().any(|a| a.is_some()) {
            Some(all_cleared_approvals)
        } else {
            None
        };

        // Call receiver with all tokens
        ext_mt_receiver::ext(receiver_id.clone())
            .with_static_gas(
                env::prepaid_gas()
                    .saturating_sub(GAS_FOR_MT_TRANSFER_CALL)
                    .saturating_sub(GAS_FOR_RESOLVE_TRANSFER),
            )
            .mt_on_transfer(
                predecessor_id,
                previous_owner_ids.clone(),
                token_ids.clone(),
                amounts.clone(),
                msg,
            )
            .then(
                ext_mt_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .mt_resolve_transfer(
                        previous_owner_ids,
                        receiver_id,
                        token_ids,
                        amounts,
                        cleared_approvals,
                    ),
            )
            .into()
    }

    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>> {
        token_ids
            .into_iter()
            .map(|token_id| {
                if !self.token_exists(&token_id) {
                    return None;
                }

                let owner_id = self.owner_by_id.get(&token_id);
                let metadata = self.token_metadata_by_id.as_ref().and_then(|m| m.get(&token_id));

                Some(Token { token_id, owner_id, metadata, approved_account_ids: None })
            })
            .collect()
    }

    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128 {
        U128(self.internal_balance_of(&account_id, &token_id))
    }

    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128> {
        token_ids
            .into_iter()
            .map(|token_id| U128(self.internal_balance_of(&account_id, &token_id)))
            .collect()
    }

    fn mt_supply(&self, token_id: TokenId) -> Option<U128> {
        self.total_supply.get(&token_id).map(U128)
    }

    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>> {
        token_ids.into_iter().map(|token_id| self.total_supply.get(&token_id).map(U128)).collect()
    }
}

impl MultiTokenResolver for MultiToken {
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
    ) -> Vec<U128> {
        require!(
            token_ids.len() == amounts.len() && token_ids.len() == previous_owner_ids.len(),
            "Invalid arguments"
        );

        // Get the result from mt_on_transfer
        #[allow(deprecated)]
        let refund_amounts: Vec<U128> = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                near_sdk::serde_json::from_slice(&value).unwrap_or_else(|_| amounts.clone())
            }
            _ => amounts.clone(), // On failure, refund everything
        };

        let mut used_amounts = Vec::with_capacity(amounts.len());

        for (i, ((token_id, original_amount), refund_amount)) in
            token_ids.iter().zip(amounts.iter()).zip(refund_amounts.iter()).enumerate()
        {
            let previous_owner_id = &previous_owner_ids[i];

            // Calculate how much was actually used vs refunded
            let refund = std::cmp::min(refund_amount.0, original_amount.0);
            let used = original_amount.0 - refund;
            used_amounts.push(U128(used));

            if refund == 0 {
                continue; // No refund needed
            }

            // Check receiver's current balance
            let receiver_balance = self.internal_balance_of(&receiver_id, token_id);
            let actual_refund = std::cmp::min(refund, receiver_balance);

            if actual_refund == 0 {
                continue;
            }

            // Transfer back to previous owner
            self.internal_set_balance(token_id, &receiver_id, receiver_balance - actual_refund);

            let previous_balance = self.internal_balance_of(previous_owner_id, token_id);
            self.internal_set_balance(
                token_id,
                previous_owner_id,
                previous_balance + actual_refund,
            );

            // Update enumeration
            if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
                // Remove from receiver if balance is now 0
                if receiver_balance - actual_refund == 0 {
                    if let Some(mut receiver_tokens) = tokens_per_owner.get(&receiver_id) {
                        receiver_tokens.remove(token_id);
                        if receiver_tokens.is_empty() {
                            tokens_per_owner.remove(&receiver_id);
                        } else {
                            tokens_per_owner.insert(&receiver_id, &receiver_tokens);
                        }
                    }
                }

                // Add back to previous owner
                let mut owner_tokens =
                    tokens_per_owner.get(previous_owner_id).unwrap_or_else(|| {
                        UnorderedSet::new(StorageKey::TokensPerOwner {
                            account_hash: env::sha256(previous_owner_id.as_bytes()),
                        })
                    });
                owner_tokens.insert(token_id);
                tokens_per_owner.insert(previous_owner_id, &owner_tokens);
            }

            // Restore approvals if provided
            if let Some(ref all_approvals) = approvals {
                if let Some(Some(token_approvals)) = all_approvals.get(i) {
                    if let Some(approvals_by_id) = &mut self.approvals_by_id {
                        let mut token_approval_map =
                            approvals_by_id.get(token_id).unwrap_or_default();
                        let mut owner_approvals =
                            token_approval_map.get(previous_owner_id).cloned().unwrap_or_default();

                        for (account_id, approval_id, amount) in token_approvals {
                            owner_approvals.insert(
                                account_id.clone(),
                                Approval { approval_id: *approval_id, amount: *amount },
                            );
                        }

                        token_approval_map.insert(previous_owner_id.clone(), owner_approvals);
                        approvals_by_id.insert(token_id, &token_approval_map);
                    }
                }
            }

            // Emit refund transfer event
            log!("Refund {} of token {} to {}", actual_refund, token_id, previous_owner_id);
        }

        used_amounts
    }
}

impl crate::multi_token::enumeration::MultiTokenEnumeration for MultiToken {
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token> {
        let start_index: u128 = from_index.map(|v| v.0).unwrap_or(0);
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);

        self.owner_by_id
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|(token_id, owner_id)| {
                let metadata = self.token_metadata_by_id.as_ref().and_then(|m| m.get(&token_id));

                Token { token_id, owner_id: Some(owner_id), metadata, approved_account_ids: None }
            })
            .collect()
    }

    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<Token> {
        let tokens_per_owner = if let Some(tokens_per_owner) = &self.tokens_per_owner {
            tokens_per_owner
        } else {
            return vec![];
        };

        let token_set = if let Some(token_set) = tokens_per_owner.get(&account_id) {
            token_set
        } else {
            return vec![];
        };

        let start_index: u128 = from_index.map(|v| v.0).unwrap_or(0);
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);

        token_set
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|token_id| {
                let owner_id = self.owner_by_id.get(&token_id);
                let metadata = self.token_metadata_by_id.as_ref().and_then(|m| m.get(&token_id));

                Token { token_id, owner_id, metadata, approved_account_ids: None }
            })
            .collect()
    }
}
