use std::collections::HashMap;

use near_sdk::{assert_one_yocto, env, log, require, AccountId, Gas, Promise};

use crate::multi_token::approval::receiver::ext_approval_receiver;
use crate::multi_token::{
    core::MultiToken,
    token::{Approval, TokenId},
    utils::{
        bytes_for_approved_account_id, expect_approval, expect_approval_for_token, refund_deposit,
        Entity,
    },
};

use super::MultiTokenApproval;

pub const GAS_FOR_RESOLVE_APPROVE: Gas = Gas::from_tgas(15);
pub const GAS_FOR_MT_APPROVE_CALL: Gas = Gas::from_tgas(50 + GAS_FOR_RESOLVE_APPROVE.as_tgas());

impl MultiTokenApproval for MultiToken {
    fn mt_approve(
        &mut self,
        token_id: TokenId,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        let approver_id = env::predecessor_account_id();

        // Unwrap to check if approval supported
        let by_token = expect_approval(self.approvals_by_token_id.as_mut(), Entity::Contract);

        // Get some IDs and check if approval management supported both for contract & token
        let next_id_by_token =
            expect_approval(self.next_approval_id_by_id.as_mut(), Entity::Contract);

        let mut used_storage = 0;

        // Get the balance to check if user has enough tokens
        let approver_balance = self
            .balances_per_token
            .get(&token_id)
            .and_then(|balances_per_account| balances_per_account.get(&approver_id))
            .unwrap_or(0);
        require!(approver_balance > 0, "Not enough balance to approve");

        // Get the next approval id for the token
        let new_approval_id: u64 =
            expect_approval_for_token(next_id_by_token.get(&token_id), &token_id);
        let new_approval = Approval { amount: approver_balance, approval_id: new_approval_id };
        log!("New approval: {:?}", new_approval);

        // Get existing approvals for this token. If one exists for the account_id, overwrite it.
        let mut by_owner = by_token.get(&token_id).unwrap_or_default();
        let by_grantee = by_owner.get(&approver_id);
        let mut grantee_to_approval =
            if let Some(by_grantee) = by_grantee { by_grantee.clone() } else { HashMap::new() };

        let old_approval_id = grantee_to_approval.insert(account_id.clone(), new_approval);
        by_owner.insert(approver_id.clone(), grantee_to_approval);
        by_token.insert(&token_id, &by_owner);
        next_id_by_token.insert(&token_id, &(new_approval_id + 1));

        log!("Updated approvals by id: {:?}", old_approval_id);
        used_storage +=
            if old_approval_id.is_none() { bytes_for_approved_account_id(&account_id) } else { 0 };

        refund_deposit(used_storage);

        // if given `msg`, schedule call to `mt_on_approve` and return it. Else, return None.
        let receiver_gas: Gas = env::prepaid_gas()
            .checked_sub(GAS_FOR_MT_APPROVE_CALL.into())
            .unwrap_or_else(|| env::panic_str("Prepaid gas overflow"))
            .into();

        msg.map(|msg| {
            ext_approval_receiver::ext(account_id).with_static_gas(receiver_gas).mt_on_approve(
                token_id,
                new_approval_id,
                msg,
            )
        })
    }

    fn mt_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
        assert_one_yocto();
        let owner_id = env::predecessor_account_id();

        // Get all approvals for token, will panic if approval extension is not used for contract or token
        let by_token = expect_approval(self.approvals_by_token_id.as_mut(), Entity::Contract);

        // Remove approval for user & also clean maps to save space it it's empty
        let mut by_owner = expect_approval_for_token(by_token.get(&token_id), &token_id);
        let by_grantee = by_owner.get_mut(&owner_id);

        if let Some(grantee_to_approval) = by_grantee {
            grantee_to_approval.remove(&account_id);
            // The owner has no more approvals for this token.
            if grantee_to_approval.is_empty() {
                by_owner.remove(&owner_id);
            }
        }

        if by_owner.is_empty() {
            by_token.remove(&token_id);
        }
    }

    fn mt_revoke_all(&mut self, token_id: TokenId) {
        assert_one_yocto();
        let owner_id = env::predecessor_account_id();

        // Get all approvals for token, will panic if approval extension is not used for contract or token
        let by_token = expect_approval(self.approvals_by_token_id.as_mut(), Entity::Contract);

        let mut by_owner = expect_approval_for_token(by_token.get(&token_id), &token_id);
        by_owner.remove(&owner_id);
        by_token.insert(&token_id, &by_owner);
    }

    fn mt_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool {
        require!(approval_id.is_some(), "approval_id must be supplied");

        let approval_id = approval_id.unwrap_or_default();

        let by_token = expect_approval(self.approvals_by_token_id.as_ref(), Entity::Contract);
        let owner_id = self.owner_by_id.get(&token_id).expect("owner not found");

        let by_owner = by_token.get(&token_id).unwrap_or_default();

        let approval = match by_owner
            .get(&owner_id)
            .and_then(|grantee_to_approval| grantee_to_approval.get(&approved_account_id))
        {
            Some(approval) => approval,
            _ => return false,
        };

        if !approval.approval_id.eq(&approval_id) {
            return false;
        }

        true
    }
}
