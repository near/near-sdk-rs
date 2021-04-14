use crate::non_fungible_token::core::NonFungibleTokenCore;
use crate::non_fungible_token::metadata::TokenMetadata;
use crate::non_fungible_token::resolver::NonFungibleTokenResolver;
use crate::non_fungible_token::token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{Base64VecU8, ValidAccountId, U128};
use near_sdk::{
    assert_one_yocto, env, ext_contract, log, AccountId, Balance, BorshStorageKey, Gas, Promise,
    PromiseResult, StorageUsage,
};
use std::collections::HashMap;

const GAS_FOR_RESOLVE_TRANSFER: Gas = 5_000_000_000_000;
const GAS_FOR_FT_TRANSFER_CALL: Gas = 25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER;

const NO_DEPOSIT: Balance = 0;

#[ext_contract(ext_self)]
trait NFTResolver {
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: Option<HashMap<AccountId, u64>>,
        token_id: TokenId,
    ) -> bool;
}

#[ext_contract(ext_receiver)]
pub trait NonFungibleTokenReceiver {
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> Promise;
}

/// Implementation of a NonFungibleToken standard.
/// Allows to include NEP-171 compatible token to any contract.
/// There are next traits that any contract may implement:
///     - NonFungibleTokenCore -- interface with nft_transfer methods. NonFungibleToken provides methods for it.
///     - NonFungibleTokenApproval -- interface with nft_approve methods. NonFungibleToken provides methods for it.
///     - NonFungibleTokenEnumeration -- interface for getting lists of tokens. NonFungibleToken provides methods for it.
///     - NonFungibleTokenMetadata -- return metadata for the token in NEP-177, up to contract to implement.
///
/// For example usage, see examples/non-fungible-token/src/lib.rs.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct NonFungibleToken {
    // owner of contract; this is the only account allowed to call `mint`
    pub owner_id: AccountId,

    // The storage size in bytes for each new token
    pub extra_storage_in_bytes_per_token: StorageUsage,

    // always required
    pub owner_by_id: UnorderedMap<TokenId, AccountId>,

    // required by metadata extension
    pub token_metadata_by_id: Option<LookupMap<TokenId, TokenMetadata>>,

    // required by enumeration extension
    pub tokens_per_owner: Option<LookupMap<AccountId, UnorderedSet<TokenId>>>,

    // required by approval extension
    pub approvals_by_id: Option<LookupMap<TokenId, HashMap<AccountId, u64>>>,
    pub next_approval_id_by_id: Option<LookupMap<TokenId, u64>>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    TokensForOwner { account_hash: Vec<u8> },
}

impl NonFungibleToken {
    pub fn new(
        owner_by_id_prefix: Vec<u8>,
        owner_id: ValidAccountId,
        token_metadata_prefix: Option<Vec<u8>>,
        enumeration_prefix: Option<Vec<u8>>,
        approval_prefix: Option<Vec<u8>>,
    ) -> Self {
        let mut this = Self {
            owner_id: owner_id.into(),
            extra_storage_in_bytes_per_token: 0,
            owner_by_id: UnorderedMap::new(owner_by_id_prefix),
            token_metadata_by_id: if let Some(prefix) = token_metadata_prefix {
                Some(LookupMap::new(prefix))
            } else {
                None
            },
            tokens_per_owner: if let Some(prefix) = enumeration_prefix {
                Some(LookupMap::new(prefix))
            } else {
                None
            },
            approvals_by_id: if let Some(prefix) = approval_prefix {
                Some(LookupMap::new(prefix))
            } else {
                None
            },
            next_approval_id_by_id: if let Some(prefix) = approval_prefix {
                Some(LookupMap::new([prefix, "n".into()].concat()))
            } else {
                None
            },
        };
        this.measure_min_token_storage_cost();
        this
    }

    // TODO: does this seem reasonable?
    fn measure_min_token_storage_cost(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let tmp_token_id = "a".repeat(64); // TODO: what's a reasonable max TokenId length?
        let tmp_owner_id = "a".repeat(64);

        // 1. set some dummy data
        self.owner_by_id.insert(&tmp_token_id, &tmp_owner_id);
        if let Some(token_metadata_by_id) = self.token_metadata_by_id {
            token_metadata_by_id.insert(
                &tmp_token_id,
                &TokenMetadata {
                    title: Some("a".repeat(64)),
                    description: Some("a".repeat(64)),
                    media: Some("a".repeat(64)),
                    media_hash: Some(Base64VecU8::from("a".repeat(64).as_bytes().to_vec())),
                    copies: Some(1),
                    issued_at: None,
                    expires_at: None,
                    starts_at: None,
                    updated_at: None,
                    extra: None,
                    reference: None,
                    reference_hash: None,
                },
            );
        }
        if let Some(tokens_per_owner) = self.tokens_per_owner {
            let token_ids = tokens_per_owner.get(&tmp_owner_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKeys::TokensForOwner {
                    account_hash: env::sha256(tmp_owner_id.as_bytes()),
                })
            });
            token_ids.insert(&tmp_token_id);
        }
        if let Some(approvals_by_id) = self.approvals_by_id {
            let mut approvals = HashMap::new();
            approvals.insert(tmp_owner_id.clone(), 1u64);
            approvals_by_id.insert(&tmp_token_id, &approvals);
        }
        if let Some(next_approval_id_by_id) = self.next_approval_id_by_id {
            next_approval_id_by_id.insert(&tmp_token_id, &1u64);
        }

        // 2. see how much space it took
        self.extra_storage_in_bytes_per_token = env::storage_usage() - initial_storage_usage;

        // 3. roll it all back
        if let Some(next_approval_id_by_id) = self.next_approval_id_by_id {
            next_approval_id_by_id.remove(&tmp_token_id);
        }
        if let Some(approvals_by_id) = self.approvals_by_id {
            approvals_by_id.remove(&tmp_token_id);
        }
        if let Some(tokens_per_owner) = self.tokens_per_owner {
            tokens_per_owner.remove(&tmp_owner_id);
        }
        if let Some(token_metadata_by_id) = self.token_metadata_by_id {
            token_metadata_by_id.remove(&tmp_token_id);
        }
        self.owner_by_id.remove(&tmp_token_id);
    }

    // Transfer from current owner to receiver_id, checking that sender is allowed to transfer.
    // Clear approvals, if approval extension being used.
    // Return previous owner and approvals.
    pub fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> (AccountId, Option<HashMap<AccountId, u64>>) {
        let owner_id = self.owner_by_id.get(token_id).expect("Token not found");

        let approved_account_ids = self.approvals_by_id.and_then(|by_id| by_id.get(&token_id));

        // check if authorized
        if sender_id != &owner_id {
            // if approval extension is NOT being used, or if token has no approved accounts
            if approved_account_ids.is_none() {
                env::panic(b"Unauthorized")
            }

            // Approval extension is being used; get approval_id for sender.
            let actual_approval_id = approved_account_ids.unwrap().get(sender_id);

            // Panic if sender not approved at all
            if actual_approval_id.is_none() {
                env::panic(b"Sender not approved");
            }

            // If approval_id included, check that it matches
            if let Some(enforced_approval_id) = approval_id {
                let actual_approval_id = actual_approval_id.unwrap();
                assert_eq!(
                    actual_approval_id, &enforced_approval_id,
                    "The actual approval_id {} is different from the given approval_id {}",
                    actual_approval_id, enforced_approval_id,
                );
            }
        }

        assert_ne!(&owner_id, receiver_id, "Current and next owner must differ");

        // update owner
        self.owner_by_id.insert(&token_id, &receiver_id);

        // if using Enumeration standard, update old & new owner's token lists
        if let Some(tokens_per_owner) = self.tokens_per_owner {
            let owner_tokens = tokens_per_owner.get(&owner_id).expect("unreachable");
            let receiver_tokens = tokens_per_owner.get(&receiver_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKeys::TokensForOwner {
                    account_hash: env::sha256(receiver_id.as_bytes()),
                })
            });
            owner_tokens.remove(&token_id);
            receiver_tokens.insert(&token_id);
        }

        // clear approvals
        let old_approvals = self.approvals_by_id.and_then(|by_id| by_id.remove(token_id));

        log!("Transfer {} from {} to {}", token_id, sender_id, receiver_id);
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }

        // return previous owner & approvals
        (owner_id, old_approvals)
    }
}

impl NonFungibleTokenCore for NonFungibleToken {
    fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        self.internal_transfer(&sender_id, receiver_id.as_ref(), &token_id, approval_id, memo);
    }

    fn nft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let (old_owner, old_approvals) =
            self.internal_transfer(&sender_id, receiver_id.as_ref(), &token_id, approval_id, memo);
        // Initiating receiver's call and the callback
        ext_receiver::nft_on_transfer(
            sender_id.clone(),
            old_owner,
            token_id,
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::nft_resolve_transfer(
            sender_id,
            receiver_id.into(),
            old_approvals,
            token_id,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    fn nft_token(self, token_id: TokenId) -> Option<Token> {
        let owner_id = self.owner_by_id.get(&token_id)?;
        let metadata = self.token_metadata_by_id.and_then(|by_id| by_id.get(&token_id));
        let approved_account_ids = self.approvals_by_id.and_then(|by_id| by_id.get(&token_id));
        Some(Token { token_id, owner_id, metadata, approved_account_ids })
    }
}

impl NonFungibleToken {
    /// Internal method that returns the amount of burned tokens in a corner case when the sender
    /// has deleted (unregistered) their account while the `nft_transfer_call` was still in flight.
    /// Returns (Used token amount, Burned token amount)
    pub fn internal_nft_resolve_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) -> (u128, u128) {
        let receiver_id: AccountId = receiver_id.into();
        let token_id: TokenId = amount.into();

        // Get the unused amount from the `nft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount > 0 {
            let receiver_balance = self.accounts.get(&receiver_id).unwrap_or(0);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                self.accounts.insert(&receiver_id, &(receiver_balance - refund_amount));

                if let Some(sender_balance) = self.accounts.get(&sender_id) {
                    self.accounts.insert(&sender_id, &(sender_balance + refund_amount));
                    log!("Refund {} from {} to {}", refund_amount, receiver_id, sender_id);
                    return (amount - refund_amount, 0);
                } else {
                    // Sender's account was deleted, so we need to burn tokens.
                    self.total_supply -= refund_amount;
                    log!("The account of the sender was deleted");
                    return (amount, refund_amount);
                }
            }
        }
        (amount, 0)
    }
}

impl NonFungibleTokenResolver for NonFungibleToken {
    fn nft_resolve_transfer(
        &mut self,
        sender_id: ValidAccountId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) -> U128 {
        self.internal_nft_resolve_transfer(sender_id.as_ref(), receiver_id, amount).0.into()
    }
}
