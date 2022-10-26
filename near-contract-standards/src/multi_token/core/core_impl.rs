use crate::multi_token::core::{MultiTokenCore, MultiTokenResolver};
use crate::multi_token::events::{MtMint, MtTransfer};
use crate::multi_token::metadata::TokenMetadata;
use crate::multi_token::token::{ApprovalContainer, ClearedApproval, Token, TokenId};
use crate::multi_token::utils::{
    check_and_apply_approval, expect_approval, refund_deposit_to_account, Entity,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, TreeMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, ext_contract, log, require, AccountId, Balance, BorshStorageKey,
    CryptoHash, Gas, IntoStorageKey, PromiseOrValue, PromiseResult, StorageUsage,
};

pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(15_000_000_000_000);
pub const GAS_FOR_MT_TRANSFER_CALL: Gas = Gas(50_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);

const NO_DEPOSIT: Balance = 0;

#[ext_contract(ext_self)]
trait MultiTokenResolver {
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
    ) -> Vec<U128>;
}

#[ext_contract(ext_multi_token_receiver)]
pub trait MultiTokenReceiver {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}

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
}

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    PerOwner,
    TokensPerOwner { account_hash: Vec<u8> },
    TokenPerOwnerInner { account_id_hash: CryptoHash },
    OwnerById,
    OwnerByIdInner { account_id_hash: CryptoHash },
    TokenMetadata,
    Approvals,
    ApprovalById,
    ApprovalsInner { account_id_hash: CryptoHash },
    TotalSupply { supply: u128 },
    Balances,
    BalancesInner { token_id: Vec<u8> },
}

impl MultiToken {
    pub fn new<Q, R, S, T>(
        _owner_by_id_prefix: Q,
        owner_id: AccountId,
        token_metadata_prefix: Option<R>,
        enumeration_prefix: Option<S>,
        approval_prefix: Option<T>,
    ) -> Self
    where
        Q: IntoStorageKey,
        R: IntoStorageKey,
        S: IntoStorageKey,
        T: IntoStorageKey,
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

        Self {
            owner_id,
            extra_storage_in_bytes_per_emission: 0,
            owner_by_id: TreeMap::new(StorageKey::OwnerById),
            total_supply: LookupMap::new(StorageKey::TotalSupply { supply: 0 }),
            token_metadata_by_id: token_metadata_prefix.map(LookupMap::new),
            tokens_per_owner: enumeration_prefix.map(LookupMap::new),
            balances_per_token: UnorderedMap::new(StorageKey::Balances),
            approvals_by_token_id,
            next_approval_id_by_id,
            next_token_id: 0,
        }
    }

    /// Used to get balance of specified account in specified token
    pub fn internal_unwrap_balance_of(
        &self,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Balance {
        match self
            .balances_per_token
            .get(token_id)
            .expect("This token does not exist")
            .get(account_id)
        {
            Some(balance) => balance,
            None => {
                env::panic_str(format!("The account {} is not registered", account_id).as_str())
            }
        }
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
            let mut balances = self.balances_per_token.get(token_id).unwrap();
            balances.insert(account_id, &new);
            self.total_supply.insert(
                token_id,
                &self
                    .total_supply
                    .get(token_id)
                    .unwrap()
                    .checked_add(amount)
                    .unwrap_or_else(|| env::panic_str("Total supply overflow")),
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
            let mut balances = self.balances_per_token.get(token_id).unwrap();
            balances.insert(account_id, &new);
            self.total_supply.insert(
                token_id,
                &self
                    .total_supply
                    .get(token_id)
                    .unwrap()
                    .checked_sub(amount)
                    .unwrap_or_else(|| env::panic_str("Total supply overflow")),
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
    ) -> (Vec<AccountId>, Vec<Option<Vec<ClearedApproval>>>) {
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
    ) -> (AccountId, Option<Vec<ClearedApproval>>) {
        // Safety checks
        require!(amount > 0, "Transferred amounts must be greater than 0");

        let (sender_id, old_approvals) = if let Some((account_id, approval_id)) = approval {
            // If an approval was provided, ensure it meets requirements.
            let approvals = expect_approval(self.approvals_by_token_id.as_mut(), Entity::Contract);

            let mut token_approvals = approvals
                .get(token_id)
                .unwrap_or_else(|| panic!("Approvals not supported for token {}", token_id));

            (
                account_id,
                Some(check_and_apply_approval(
                    &mut token_approvals,
                    account_id,
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

    pub fn internal_register_account(&mut self, token_id: &TokenId, account_id: &AccountId) {
        if self.balances_per_token.get(token_id).unwrap().insert(account_id, &0).is_some() {
            env::panic_str("The account is already registered");
        }
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
        self.token_metadata_by_id
            .as_mut()
            .and_then(|by_id| by_id.insert(&token_id, &token_metadata.clone().unwrap()));

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
        require!(
            env::prepaid_gas() > GAS_FOR_MT_TRANSFER_CALL + GAS_FOR_RESOLVE_TRANSFER,
            "Insufficient gas for transfer"
        );
        let sender_id = env::predecessor_account_id();

        let amount_to_send: Balance = amount.0;

        let (old_owner, old_approvals) =
            self.internal_transfer(&sender_id, &receiver_id, &token_id, amount_to_send, &approval);

        ext_multi_token_receiver::mt_on_transfer(
            sender_id,
            vec![old_owner.clone()],
            vec![token_id.clone()],
            vec![amount],
            msg,
            receiver_id.clone(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_MT_TRANSFER_CALL,
        )
        .then(ext_self::mt_resolve_transfer(
            vec![old_owner],
            receiver_id,
            vec![token_id],
            vec![amount],
            Some(vec![old_approvals]),
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
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
        require!(
            env::prepaid_gas() > GAS_FOR_MT_TRANSFER_CALL + GAS_FOR_RESOLVE_TRANSFER,
            "Insufficient gas for transfer"
        );
        let sender_id = env::predecessor_account_id();

        let amounts_to_send: Vec<Balance> = amounts.iter().map(|x| x.0).collect();

        let (old_owners, old_approvals) = self.internal_batch_transfer(
            &sender_id,
            &receiver_id,
            &token_ids,
            &amounts_to_send,
            approvals,
        );

        ext_multi_token_receiver::mt_on_transfer(
            sender_id,
            old_owners.clone(),
            token_ids.clone(),
            amounts.clone(),
            msg,
            receiver_id.clone(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_MT_TRANSFER_CALL,
        )
        .then(ext_self::mt_resolve_transfer(
            old_owners,
            receiver_id,
            token_ids,
            amounts,
            Some(old_approvals),
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>> {
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
        _approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
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
            // Upstream call failed and transfers have been reverted.
            // TODO: Re-grant approvals so client may try again.
        }

        amounts_kept_by_receiver
    }

    pub fn internal_resolve_single_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver: AccountId,
        token_id: TokenId,
        amount: u128,
        to_refund: u128,
    ) -> Balance {
        if to_refund > 0 {
            // Whatever was unused gets returned to the original owner.
            let mut balances = self.balances_per_token.get(&token_id).unwrap();
            let receiver_balance = balances.get(&receiver).unwrap_or(0);

            if receiver_balance > 0 {
                // If the receiver doesn't have enough funds to do the
                // full refund, just refund all that we can.
                let refund = std::cmp::min(receiver_balance, to_refund);
                balances.insert(&receiver, &(receiver_balance - refund));

                // Try to give the refund back to sender now
                return if let Some(sender_balance) = balances.get(sender_id) {
                    balances.insert(sender_id, &(sender_balance + refund));
                    log!("Refund {} from {} to {}", refund, receiver, sender_id);
                    amount - refund
                } else {
                    *self.total_supply.get(&token_id).as_mut().unwrap() -= refund;
                    log!("The account of the sender was deleted");
                    amount - refund
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
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
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
