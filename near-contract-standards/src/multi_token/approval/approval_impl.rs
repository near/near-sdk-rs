//! Implementation of approval management for MultiToken.

use crate::multi_token::approval::MultiTokenApproval;
use crate::multi_token::core::{MultiToken, approval_key};
use crate::multi_token::token::{Approval, TokenId};
use crate::multi_token::utils::{
    assert_at_least_one_yocto, refund_approved_account_ids_iter, refund_deposit,
};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, Gas, Promise, env, require};

const GAS_FOR_MT_ON_APPROVE: Gas = Gas::from_tgas(35);

impl MultiTokenApproval for MultiToken {
    fn mt_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        assert_at_least_one_yocto();
        require!(
            token_ids.len() == amounts.len(),
            "token_ids and amounts must have the same length"
        );

        require!(self.approvals_by_id.is_some(), "Approval extension not enabled");

        let owner_id = env::predecessor_account_id();
        let initial_storage = env::storage_usage();

        // First, verify all balances (immutable borrow)
        for (token_id, amount) in token_ids.iter().zip(amounts.iter()) {
            let balance = self.internal_balance_of(&owner_id, token_id);
            require!(
                balance >= amount.0,
                format!(
                    "Cannot approve more than owned. Balance: {}, Approval amount: {}",
                    balance, amount.0
                )
            );
        }

        // Now do the mutable operations
        let mut approval_ids = Vec::with_capacity(token_ids.len());

        for (token_id, amount) in token_ids.iter().zip(amounts.iter()) {
            // Get next approval ID
            let next_approval_id_by_id =
                self.next_approval_id_by_id.as_mut().expect("Approval extension not enabled");
            let approval_id = next_approval_id_by_id.get(token_id).unwrap_or(0);
            let new_approval_id = approval_id + 1;
            next_approval_id_by_id.insert(token_id, &new_approval_id);

            // Get or create approval map for this (token, owner) pair
            let approvals_by_id =
                self.approvals_by_id.as_mut().expect("Approval extension not enabled");
            let akey = approval_key(token_id, &owner_id);
            let mut owner_approvals = approvals_by_id.get(&akey).unwrap_or_default();

            // Insert approval
            owner_approvals.insert(
                account_id.clone(),
                Approval { approval_id: new_approval_id, amount: amount.0 },
            );
            approvals_by_id.insert(&akey, &owner_approvals);

            approval_ids.push(new_approval_id);
        }

        // Refund unused deposit
        let storage_used = env::storage_usage() - initial_storage;
        refund_deposit(storage_used);

        // Call mt_on_approve if msg is provided
        msg.map(|msg| {
            super::ext_mt_approval_receiver::ext(account_id)
                .with_static_gas(GAS_FOR_MT_ON_APPROVE)
                .mt_on_approve(token_ids, amounts, owner_id, approval_ids, msg)
        })
    }

    fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId) {
        assert_at_least_one_yocto();

        let approvals_by_id =
            self.approvals_by_id.as_mut().expect("Approval extension not enabled");

        let owner_id = env::predecessor_account_id();
        let mut revoked_any = false;

        for token_id in token_ids {
            let akey = approval_key(&token_id, &owner_id);
            if let Some(mut owner_approvals) = approvals_by_id.get(&akey) {
                if owner_approvals.remove(&account_id).is_some() {
                    revoked_any = true;
                    if owner_approvals.is_empty() {
                        approvals_by_id.remove(&akey);
                    } else {
                        approvals_by_id.insert(&akey, &owner_approvals);
                    }
                }
            }
        }

        // Refund storage costs for the removed approval
        if revoked_any {
            refund_approved_account_ids_iter(owner_id, core::iter::once(&account_id)).detach();
        }
    }

    fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>) {
        assert_at_least_one_yocto();

        let approvals_by_id =
            self.approvals_by_id.as_mut().expect("Approval extension not enabled");

        let owner_id = env::predecessor_account_id();
        let mut all_revoked_accounts: Vec<AccountId> = Vec::new();

        for token_id in token_ids {
            let akey = approval_key(&token_id, &owner_id);
            if let Some(owner_approvals) = approvals_by_id.remove(&akey) {
                // Collect all revoked account IDs for refund
                all_revoked_accounts.extend(owner_approvals.keys().cloned());
            }
        }

        // Refund storage costs for all removed approvals
        if !all_revoked_accounts.is_empty() {
            refund_approved_account_ids_iter(owner_id, all_revoked_accounts.iter()).detach();
        }
    }

    fn mt_is_approved(
        &self,
        token_ids: Vec<TokenId>,
        owner_id: AccountId,
        approved_account_id: AccountId,
        amounts: Vec<U128>,
        approval_ids: Option<Vec<u64>>,
    ) -> bool {
        require!(
            token_ids.len() == amounts.len(),
            "token_ids and amounts must have the same length"
        );
        if let Some(ref ids) = approval_ids {
            require!(
                token_ids.len() == ids.len(),
                "approval_ids must have the same length as token_ids"
            );
        }

        let approvals_by_id = match &self.approvals_by_id {
            Some(approvals) => approvals,
            None => return false,
        };

        for (i, (token_id, amount)) in token_ids.iter().zip(amounts.iter()).enumerate() {
            let akey = approval_key(token_id, &owner_id);
            let owner_approvals = match approvals_by_id.get(&akey) {
                Some(approvals) => approvals,
                None => return false,
            };

            match owner_approvals.get(&approved_account_id) {
                Some(approval) => {
                    // Check amount
                    if approval.amount < amount.0 {
                        return false;
                    }
                    // Check approval_id if provided
                    if let Some(ref ids) = approval_ids {
                        if approval.approval_id != ids[i] {
                            return false;
                        }
                    }
                }
                None => return false,
            }
        }

        true
    }
}
