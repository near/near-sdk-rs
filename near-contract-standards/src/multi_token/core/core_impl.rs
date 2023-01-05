use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, TreeMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, log, require, AccountId, Balance, BorshStorageKey, CryptoHash, Gas,
    IntoStorageKey, PromiseOrValue, PromiseResult, StorageUsage,
};

use crate::multi_token::core::receiver::ext_mt_receiver;
use crate::multi_token::core::resolver::{ext_mt_resolver, MultiTokenResolver};
use crate::multi_token::core::MultiTokenCore;
use crate::multi_token::events::{MtMint, MtTransfer};
use crate::multi_token::metadata::TokenMetadata;
use crate::multi_token::token::{Approval, ApprovalContainer, ClearedApproval, Token, TokenId};
use crate::multi_token::utils::{
    expect_approval, expect_approval_for_token, refund_deposit_to_account, Entity,
};

pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(15_000_000_000_000);
pub const GAS_FOR_MT_TRANSFER_CALL: Gas = Gas(50_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);

const ERR_MORE_GAS_REQUIRED: &str = "More gas is required";
const ERR_TOTAL_SUPPLY_OVERFLOW: &str = "Total supply overflow";
const ERR_PREPAID_GAS_OVERFLOW: &str = "Prepaid gas overflow";
const ERR_TOTAL_SUPPLY_NOT_FOUND_BY_TOKEN_ID: &str = "Total supply not found by token id";

/// Implementation of the multi-token standard
/// Allows to include NEP-245 compatible tokens to any contract.
/// There are next traits that any contract may implement:
///     - MultiTokenCore -- interface with transfer methods. MultiToken provides methods for it.
///     - MultiTokenApproval -- interface with approve methods. MultiToken provides methods for it.
///     - MultiTokenEnumeration -- interface for getting lists of tokens. MultiToken provides methods for it.
///     - MultiTokenMetadata -- return metadata for the token in NEP-245, up to contract to implement.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct MultiToken {
    /// Owner of contract
    pub owner_id: AccountId,

    /// AccountID -> Near balance for storage.
    pub accounts_storage: LookupMap<AccountId, Balance>,

    /// The storage size in bytes for one account.
    pub account_storage_usage: StorageUsage,

    /// The storage size in bytes for one token.
    pub storage_usage_per_token: StorageUsage,

    /// How much storage takes every token
    pub extra_storage_in_bytes_per_emission: StorageUsage,

    /// Owner of each token
    pub owner_by_id: TreeMap<TokenId, AccountId>,

    /// Total supply for each token
    pub total_supply: LookupMap<TokenId, Balance>,

    /// Metadata for each token
    pub token_metadata_by_id: Option<LookupMap<TokenId, TokenMetadata>>,

    /// All tokens owned by user
    pub tokens_per_owner: Option<LookupMap<AccountId, UnorderedSet<TokenId>>>,

    /// Balance of user for given token
    pub balances_per_token: UnorderedMap<TokenId, LookupMap<AccountId, u128>>,
    /// Approvals granted for a given token.
    /// Nested maps are structured as: token_id -> owner_id -> grantee_id -> (approval_id, amount)
    pub approvals_by_token_id: Option<ApprovalContainer>,

    /// Next id of approval
    pub next_approval_id_by_id: Option<LookupMap<TokenId, u64>>,

    /// Next id for token
    pub next_token_id: u64,

    /// Token holders with positive balance per token
    pub holders_per_token: Option<UnorderedMap<TokenId, UnorderedSet<AccountId>>>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Accounts,
    AccountTokens { account_id_hash: CryptoHash },
    PerOwner,
    TokensPerOwner { account_hash: Vec<u8> },
    TokenPerOwnerInner { account_id_hash: CryptoHash },
    OwnerByIdInner { account_id_hash: CryptoHash },
    TokenMetadata,
    Approvals,
    ApprovalById,
    ApprovalsInner { account_id_hash: CryptoHash },
    TotalSupply { supply: u128 },
    Balances,
    BalancesInner { token_id: Vec<u8> },
    TokenHoldersInner { token_id: TokenId },
}

impl MultiToken {
    pub fn new<Q, R, S, T, U>(
        owner_by_id_prefix: Q,
        owner_id: AccountId,
        token_metadata_prefix: Option<R>,
        enumeration_prefix: Option<S>,
        approval_prefix: Option<T>,
        token_holders_prefix: Option<U>,
    ) -> Self
    where
        Q: IntoStorageKey,
        R: IntoStorageKey,
        S: IntoStorageKey,
        T: IntoStorageKey,
        U: IntoStorageKey,
    {
        let (approvals_by_token_id, next_approval_id_by_id) = if let Some(prefix) = approval_prefix
        {
            let prefix: Vec<u8> = prefix.into_storage_key();
            (
                Some(LookupMap::new(prefix.clone())),
                Some(LookupMap::new([prefix, "n".into()].concat())),
            )
        } else {
            (None, None)
        };

        let mut this = Self {
            owner_id,
            extra_storage_in_bytes_per_emission: 0,
            owner_by_id: TreeMap::new(owner_by_id_prefix),
            total_supply: LookupMap::new(StorageKey::TotalSupply { supply: 0 }),
            token_metadata_by_id: token_metadata_prefix.map(LookupMap::new),
            tokens_per_owner: enumeration_prefix.map(LookupMap::new),
            accounts_storage: LookupMap::new(StorageKey::Accounts),
            balances_per_token: UnorderedMap::new(StorageKey::Balances),
            approvals_by_token_id,
            next_approval_id_by_id,
            next_token_id: 0,
            account_storage_usage: 0,
            storage_usage_per_token: 0,
            holders_per_token: token_holders_prefix.map(UnorderedMap::new),
        };

        this.measure_min_account_storage_cost();
        this.measure_min_token_storage_cost();

        this
    }

    fn measure_min_token_storage_cost(&mut self) {
        let tmp_token_id = u64::MAX.to_string();
        let mut user_token_balance = LookupMap::new(StorageKey::BalancesInner {
            token_id: env::sha256(tmp_token_id.as_bytes()),
        });
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64));

        let initial_storage_usage = env::storage_usage();

        user_token_balance.insert(&tmp_account_id, &u128::MAX);

        self.balances_per_token.insert(&tmp_token_id, &user_token_balance);
        if let Some(holders) = &mut self.holders_per_token {
            let mut holders_set =
                UnorderedSet::new(StorageKey::TokenHoldersInner { token_id: tmp_token_id.clone() });
            holders_set.insert(&tmp_account_id);
            holders.insert(&tmp_token_id, &holders_set);
        }
        self.storage_usage_per_token = env::storage_usage() - initial_storage_usage;

        self.balances_per_token.remove(&tmp_token_id);
    }

    fn measure_min_account_storage_cost(&mut self) {
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64));
        let initial_storage_usage = env::storage_usage();

        // storage in NEAR's per account
        self.accounts_storage.insert(&tmp_account_id, &u128::MAX);

        self.account_storage_usage = env::storage_usage() - initial_storage_usage;

        self.accounts_storage.remove(&tmp_account_id);
    }

    /// Used to get balance of specified account in specified token
    pub fn internal_unwrap_balance_of(
        &self,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Balance {
        self.balances_per_token
            .get(token_id)
            .expect("This token does not exist")
            .get(account_id)
            .unwrap_or(0)
    }

    /// Add to balance of user specified amount
    pub fn internal_deposit(
        &mut self,
        token_id: &TokenId,
        account_id: &AccountId,
        amount: Balance,
    ) {
        let balance = self.internal_unwrap_balance_of(token_id, account_id);
        if let Some(new) = balance.checked_add(amount) {
            let mut balances = self.balances_per_token.get(token_id).expect("Token not found");
            balances.insert(account_id, &new);
            self.total_supply.insert(
                token_id,
                &self
                    .total_supply
                    .get(token_id)
                    .expect(ERR_TOTAL_SUPPLY_NOT_FOUND_BY_TOKEN_ID)
                    .checked_add(amount)
                    .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW)),
            );
        } else {
            env::panic_str("Balance overflow");
        }
    }

    /// Subtract specified amount from user account in given token
    pub fn internal_withdraw(
        &mut self,
        token_id: &TokenId,
        account_id: &AccountId,
        amount: Balance,
    ) {
        let balance = self.internal_unwrap_balance_of(token_id, account_id);
        if let Some(new) = balance.checked_sub(amount) {
            let mut balances = self.balances_per_token.get(token_id).expect("Token not found");
            balances.insert(account_id, &new);
            self.total_supply.insert(
                token_id,
                &self
                    .total_supply
                    .get(token_id)
                    .expect(ERR_TOTAL_SUPPLY_NOT_FOUND_BY_TOKEN_ID)
                    .checked_sub(amount)
                    .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW)),
            );
        } else {
            env::panic_str("The account doesn't have enough balance");
        }
    }

    pub fn internal_batch_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_ids: &[TokenId],
        amounts: &[Balance],
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
    ) -> (Vec<AccountId>, Vec<Option<(AccountId, Approval)>>) {
        let approvals = approvals.unwrap_or_else(|| vec![None; token_ids.len()]);
        (0..token_ids.len())
            .map(|i| {
                self.internal_transfer(
                    sender_id,
                    receiver_id,
                    &token_ids[i],
                    amounts[i],
                    &approvals[i],
                )
            })
            .unzip()
    }

    pub fn internal_transfer(
        &mut self,
        original_sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: Balance,
        approval: &Option<(AccountId, u64)>,
    ) -> (AccountId, Option<(AccountId, Approval)>) {
        // Safety checks
        require!(amount > 0, "Transferred amounts must be greater than 0");

        let (sender_id, old_approvals) = if let Some((owner_id, approval_id)) = approval {
            (
                owner_id,
                Some(self.check_and_apply_approval(
                    token_id,
                    owner_id,
                    original_sender_id,
                    approval_id,
                    amount,
                )),
            )
        } else {
            // No approval.
            (original_sender_id, None)
        };

        require!(sender_id != receiver_id, "Sender and receiver must differ");

        self.internal_withdraw(token_id, sender_id, amount);
        self.internal_deposit(token_id, receiver_id, amount);
        self.assert_storage_usage(receiver_id);
        self.internal_update_token_holders(token_id, receiver_id);
        self.internal_update_token_holders(token_id, sender_id);

        MtTransfer {
            old_owner_id: sender_id,
            new_owner_id: receiver_id,
            token_ids: &[token_id],
            amounts: &[&amount.to_string()],
            authorized_id: Some(original_sender_id).filter(|id| *id == sender_id),
            memo: None,
        }
        .emit();

        (sender_id.to_owned(), old_approvals)
    }

    pub fn internal_mint(
        &mut self,
        owner_id: AccountId,
        supply: Option<Balance>,
        metadata: Option<TokenMetadata>,
        refund_id: Option<AccountId>,
    ) -> Token {
        let token = self.internal_mint_with_refund(owner_id.clone(), supply, metadata, refund_id);
        MtMint {
            owner_id: &owner_id,
            token_ids: &[&token.token_id],
            amounts: &[&token.supply.to_string()],
            memo: None,
        }
        .emit();

        token
    }

    /// Mint a new token without checking:
    /// * Whether the caller id is equal to the `owner_id`
    /// * `refund_id` will transfer the leftover balance after storage costs are calculated to the provided account.
    ///   Typically, the account will be the owner. If `None`, will not refund. This is useful for delaying refunding
    ///   until multiple tokens have been minted.
    ///
    /// Returns the newly minted token and does not emit the mint event. This allows minting multiple before emitting.
    pub fn internal_mint_with_refund(
        &mut self,
        token_owner_id: AccountId,
        supply: Option<Balance>,
        token_metadata: Option<TokenMetadata>,
        refund_id: Option<AccountId>,
    ) -> Token {
        // Remember current storage usage if refund_id is Some
        let initial_storage_usage = refund_id.map(|account_id| (account_id, env::storage_usage()));

        // Panic if contract is using metadata extension and caller must provide it
        if self.token_metadata_by_id.is_some() && token_metadata.is_none() {
            env::panic_str("MUST provide metadata");
        }
        // Increment next id of the token. Panic if it's overflowing u64::MAX
        self.next_token_id =
            self.next_token_id.checked_add(1).expect("u64 overflow, cannot mint any more tokens");

        let token_id: TokenId = self.next_token_id.to_string();

        // If contract uses approval management create new LookupMap for approvals
        self.next_approval_id_by_id.as_mut().and_then(|internal| internal.insert(&token_id, &0));

        // Alias
        let owner_id: AccountId = token_owner_id;

        // Insert new owner
        self.owner_by_id.insert(&token_id, &owner_id);

        // Insert new metadata
        if let Some(metadata) = &token_metadata {
            self.token_metadata_by_id.as_mut().and_then(|by_id| by_id.insert(&token_id, metadata));
        }

        // Insert new supply
        let supply = supply.unwrap_or(0);
        self.total_supply.insert(&token_id, &supply);

        // Insert new balance
        let mut new_balances_per_account: LookupMap<AccountId, u128> =
            LookupMap::new(StorageKey::BalancesInner {
                token_id: env::sha256(token_id.as_bytes()),
            });
        new_balances_per_account.insert(&owner_id, &supply);
        self.balances_per_token.insert(&token_id, &new_balances_per_account);

        self.internal_update_token_holders(&token_id, &owner_id);

        // Updates enumeration if extension is used
        if let Some(per_owner) = &mut self.tokens_per_owner {
            let mut token_ids = per_owner.get(&owner_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::TokensPerOwner {
                    account_hash: env::sha256(owner_id.as_bytes()),
                })
            });
            token_ids.insert(&token_id);
            per_owner.insert(&owner_id, &token_ids);
        }

        if let Some((id, usage)) = initial_storage_usage {
            refund_deposit_to_account(env::storage_usage() - usage, id);
        }

        Token { token_id, owner_id, supply, metadata: token_metadata }
    }

    // validate that an approval exists with matching approval_id and sufficient balance.
    pub fn check_and_apply_approval(
        &mut self,
        token_id: &TokenId,
        owner_id: &AccountId,
        grantee_id: &AccountId,
        approval_id: &u64,
        amount: Balance,
    ) -> (AccountId, Approval) {
        // If an approval was provided, ensure it meets requirements.
        let approvals = expect_approval(self.approvals_by_token_id.as_mut(), Entity::Contract);

        let mut by_owner = expect_approval_for_token(approvals.get(token_id), token_id);

        let mut by_sender_id = by_owner
            .get(owner_id)
            .unwrap_or_else(|| panic!("No approvals for {}", owner_id))
            .clone();

        let stored_approval: Approval = by_sender_id
            .get(grantee_id)
            .unwrap_or_else(|| panic!("No approval for {} from {}", grantee_id, owner_id))
            .clone();

        require!(stored_approval.approval_id.eq(approval_id), "Invalid approval_id");

        let new_approval_amount = stored_approval
            .amount
            .checked_sub(amount)
            .expect("Not enough approval amount for transfer");

        if new_approval_amount == 0 {
            by_sender_id.remove(grantee_id);
        } else {
            by_sender_id.insert(
                grantee_id.clone(),
                Approval { approval_id: *approval_id, amount: new_approval_amount },
            );
        }
        by_owner.insert(owner_id.clone(), by_sender_id.clone());

        approvals.insert(token_id, &by_owner);

        // Given that we are consuming the approval or the part of it
        // Return the now-deleted approvals, so that caller may restore them in case of revert.
        (grantee_id.clone(), stored_approval)
    }

    /// Used to update the set of current holders of a token.
    pub fn internal_update_token_holders(&mut self, token_id: &TokenId, account_id: &AccountId) {
        if let Some(token_holders_by_token) = self.holders_per_token.as_mut() {
            let mut holders = token_holders_by_token.get(token_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::TokenHoldersInner { token_id: token_id.clone() })
            });
            let balances = self.balances_per_token.get(token_id).expect("Token not found");

            let account_balance = balances.get(account_id).expect("Account not found");

            if account_balance == 0 {
                holders.remove(account_id);
            } else if !holders.contains(account_id) {
                holders.insert(account_id);
            } else {
                return;
            }

            token_holders_by_token.insert(token_id, &holders);
        }
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
        self.mt_batch_transfer(
            receiver_id,
            vec![token_id],
            vec![amount],
            Some(vec![approval]),
            memo,
        );
    }

    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        _memo: Option<String>,
    ) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        require!(token_ids.len() == amounts.len());
        require!(!token_ids.is_empty());

        let amounts: Vec<Balance> = amounts.iter().map(|x| x.0).collect();

        self.internal_batch_transfer(&sender_id, &receiver_id, &token_ids, &amounts, approvals);
    }

    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        _memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        require!(env::prepaid_gas() > GAS_FOR_MT_TRANSFER_CALL, ERR_MORE_GAS_REQUIRED);
        let sender_id = env::predecessor_account_id();

        let amount_to_send: Balance = amount.0;

        let (old_owner, old_approvals) =
            self.internal_transfer(&sender_id, &receiver_id, &token_id, amount_to_send, &approval);

        let receiver_gas = env::prepaid_gas()
            .0
            .checked_sub(GAS_FOR_MT_TRANSFER_CALL.0)
            .unwrap_or_else(|| env::panic_str(ERR_PREPAID_GAS_OVERFLOW));

        ext_mt_receiver::ext(receiver_id.clone())
            .with_static_gas(receiver_gas.into())
            .mt_on_transfer(
                sender_id.clone(),
                vec![old_owner.clone()],
                vec![token_id.clone()],
                vec![amount],
                msg,
            )
            .then(
                ext_mt_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .mt_resolve_transfer(
                        vec![old_owner],
                        receiver_id,
                        vec![token_id],
                        vec![amount],
                        Some(vec![old_approvals]),
                    ),
            )
            .into()
    }

    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        _memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        require!(env::prepaid_gas() > GAS_FOR_MT_TRANSFER_CALL, ERR_MORE_GAS_REQUIRED);
        let sender_id = env::predecessor_account_id();

        let amounts_to_send: Vec<Balance> = amounts.iter().map(|x| x.0).collect();

        let (old_owners, old_approvals) = self.internal_batch_transfer(
            &sender_id,
            &receiver_id,
            &token_ids,
            &amounts_to_send,
            approvals,
        );

        let receiver_gas = env::prepaid_gas()
            .0
            .checked_sub(GAS_FOR_MT_TRANSFER_CALL.into())
            .unwrap_or_else(|| env::panic_str(ERR_PREPAID_GAS_OVERFLOW));

        ext_mt_receiver::ext(receiver_id.clone())
            .with_static_gas(receiver_gas.into())
            .mt_on_transfer(sender_id, old_owners.clone(), token_ids.clone(), amounts.clone(), msg)
            .then(
                ext_mt_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .mt_resolve_transfer(
                        old_owners,
                        receiver_id,
                        token_ids,
                        amounts,
                        Some(old_approvals),
                    ),
            )
            .into()
    }

    fn mt_token(&self, token_id: TokenId) -> Option<Token> {
        self.internal_get_token_metadata(&token_id)
    }

    fn mt_token_list(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>> {
        token_ids.iter().map(|token_id| self.internal_get_token_metadata(token_id)).collect()
    }

    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128 {
        self.internal_balance_of(&account_id, &token_id)
    }

    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128> {
        token_ids.iter().map(|token_id| self.internal_balance_of(&account_id, token_id)).collect()
    }

    fn mt_supply(&self, token_id: TokenId) -> Option<U128> {
        self.internal_supply(&token_id)
    }

    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>> {
        token_ids.iter().map(|token_id| self.internal_supply(token_id)).collect()
    }
}

impl MultiToken {
    fn internal_get_token_metadata(&self, token_id: &TokenId) -> Option<Token> {
        let metadata = if let Some(metadata_by_id) = &self.token_metadata_by_id {
            metadata_by_id.get(token_id)
        } else {
            None
        };
        let supply = self.total_supply.get(token_id)?;
        let owner_id = self.owner_by_id.get(token_id)?;

        Some(Token { token_id: token_id.clone(), owner_id, supply, metadata })
    }

    fn internal_balance_of(&self, account_id: &AccountId, token_id: &TokenId) -> U128 {
        let token_balances_by_user =
            self.balances_per_token.get(token_id).expect("Token not found.");
        token_balances_by_user.get(account_id).unwrap_or(0).into()
    }

    fn internal_supply(&self, token_id: &TokenId) -> Option<U128> {
        self.total_supply.get(token_id).map(u128::into)
    }

    pub fn internal_resolve_transfers(
        &mut self,
        previous_owner_ids: &[AccountId],
        receiver: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<ClearedApproval>>>,
    ) -> Vec<Balance> {
        // promise result contains what amounts were refunded by the receiver contract.
        let (amounts_to_refund, revert_approvals): (Vec<U128>, bool) = match env::promise_result(0)
        {
            PromiseResult::NotReady => env::abort(),
            PromiseResult::Successful(values) => {
                if let Ok(unused) = near_sdk::serde_json::from_slice::<Vec<U128>>(&values) {
                    // we can't be refunded by more than what we sent over
                    (
                        (0..amounts.len())
                            .map(|i| U128(std::cmp::min(amounts[i].0, unused[i].0)))
                            .collect(),
                        false,
                    )
                } else {
                    // Can't parse. Refund the transfers, but don't restore the approvals for the non-compliant contract.
                    (amounts.clone(), false)
                }
            }
            // If promise chain fails, undo all the transfers.
            PromiseResult::Failed => (amounts.clone(), true),
        };

        let amounts_kept_by_receiver: Vec<Balance> = (0..token_ids.len())
            .map(|i| {
                self.internal_resolve_single_transfer(
                    &previous_owner_ids[i],
                    receiver.clone(),
                    token_ids[i].clone(),
                    amounts[i].into(),
                    amounts_to_refund[i].into(),
                )
            })
            .collect();

        if revert_approvals {
            log!("Reverting approvals");

            if let Some(by_token) = self.approvals_by_token_id.as_mut() {
                if let Some(approvals) = approvals {
                    for (i, approval) in approvals.iter().enumerate() {
                        if let Some(cleared_approval) = approval {
                            let token_id = &token_ids[i];
                            let previous_owner = &previous_owner_ids[i];
                            let mut by_owner = by_token.get(token_id).expect("Token not found");
                            let by_grantee =
                                by_owner.get_mut(previous_owner).expect("Previous owner not found");
                            let (grantee_id, apprioval) = cleared_approval;
                            log!("Restored approval for token {:?} for owner {:?} and grantee {:?} with allowance {:?}", &token_id, &previous_owner, &grantee_id, &apprioval.amount);
                            by_grantee.insert(grantee_id.clone(), apprioval.clone());
                            by_token.insert(token_id, &by_owner);
                        }
                    }
                }
            }
        }

        amounts_kept_by_receiver
    }

    pub fn internal_resolve_single_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver: AccountId,
        token_id: TokenId,
        amount: u128,
        unused_amount: u128,
    ) -> Balance {
        if unused_amount > 0 {
            // Whatever was unused gets returned to the original owner.
            let mut balances = self.balances_per_token.get(&token_id).expect("Token not found");
            let receiver_balance = balances.get(&receiver).unwrap_or(0);

            if receiver_balance > 0 {
                // If the receiver doesn't have enough funds to do the
                // full refund, just refund all that we can.
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);

                if let Some(new_receiver_balance) = receiver_balance.checked_sub(refund_amount) {
                    balances.insert(&receiver, &new_receiver_balance);
                } else {
                    env::panic_str("The receiver account doesn't have enough balance");
                }

                // Try to give the refund back to sender now
                return if let Some(sender_balance) = balances.get(sender_id) {
                    if let Some(new_sender_balance) = sender_balance.checked_add(refund_amount) {
                        balances.insert(sender_id, &new_sender_balance);
                        log!("Refund {} from {} to {}", refund_amount, receiver, sender_id);
                        MtTransfer {
                            old_owner_id: sender_id,
                            new_owner_id: &receiver,
                            token_ids: &[&token_id],
                            amounts: &[&amount.to_string()],
                            authorized_id: None,
                            memo: None,
                        }
                        .emit();
                        amount
                            .checked_sub(refund_amount)
                            .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW))
                    } else {
                        env::panic_str("Sender balance overflow");
                    }
                } else {
                    self.total_supply
                        .get(&token_id)
                        .as_mut()
                        .expect(ERR_TOTAL_SUPPLY_NOT_FOUND_BY_TOKEN_ID)
                        .checked_sub(refund_amount)
                        .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW));

                    log!("The account of the sender was deleted");
                    amount
                        .checked_sub(refund_amount)
                        .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW))
                };
            }
        }
        amount
    }
}

impl MultiTokenResolver for MultiToken {
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<ClearedApproval>>>,
    ) -> Vec<U128> {
        self.internal_resolve_transfers(
            &previous_owner_ids,
            receiver_id,
            token_ids,
            amounts,
            approvals,
        )
        .iter()
        .map(|&x| x.into())
        .collect()
    }
}
