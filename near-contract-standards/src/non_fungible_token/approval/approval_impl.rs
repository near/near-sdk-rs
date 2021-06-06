/// Common implementation of the [approval management standard](https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html) for NFTs.
/// on the contract/account that has just been approved. This is not required to implement.
use crate::non_fungible_token::approval::NonFungibleTokenApproval;
use crate::non_fungible_token::token::TokenId;
use crate::non_fungible_token::utils::{
    assert_at_least_one_yocto, bytes_for_approved_account_id, refund_approved_account_ids,
    refund_approved_account_ids_iter, refund_deposit,
};
use crate::non_fungible_token::NonFungibleToken;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{assert_one_yocto, env, ext_contract, AccountId, Balance, Gas, Promise};
use std::collections::HashMap;

const GAS_FOR_NFT_APPROVE: Gas = 10_000_000_000_000;
const NO_DEPOSIT: Balance = 0;

#[ext_contract(ext_approval_receiver)]
pub trait NonFungibleTokenReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    );
}

impl NonFungibleTokenApproval for NonFungibleToken {
    fn nft_approve(
        &mut self,
        token_id: TokenId,
        account_id: ValidAccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        assert_at_least_one_yocto();
        if self.approvals_by_id.is_none() {
            env::panic(b"NFT does not support Approval Management");
        }

        let owner_id = self.owner_by_id.get(&token_id).expect("Token not found");

        assert_eq!(&env::predecessor_account_id(), &owner_id, "Predecessor must be token owner.");

        // get contract-level LookupMap of token_id to approvals HashMap
        let approvals_by_id = self.approvals_by_id.as_mut().unwrap();

        // update HashMap of approvals for this token
        let approved_account_ids =
            &mut approvals_by_id.get(&token_id).unwrap_or_else(|| HashMap::new());
        let account_id: AccountId = account_id.into();
        let approval_id: u64 =
            self.next_approval_id_by_id.as_ref().unwrap().get(&token_id).unwrap_or_else(|| 1u64);
        let old_approval_id = approved_account_ids.insert(account_id.clone(), approval_id);

        // save updated approvals HashMap to contract's LookupMap
        approvals_by_id.insert(&token_id, &approved_account_ids);

        // increment next_approval_id for this token
        self.next_approval_id_by_id.as_mut().unwrap().insert(&token_id, &(approval_id + 1));

        // If this approval replaced existing for same account, no storage was used.
        // Otherwise, require that enough deposit was attached to pay for storage, and refund
        // excess.
        let storage_used =
            if old_approval_id.is_none() { bytes_for_approved_account_id(&account_id) } else { 0 };
        refund_deposit(storage_used);

        // if given `msg`, schedule call to `nft_on_approve` and return it. Else, return None.
        if let Some(msg) = msg {
            Some(ext_approval_receiver::nft_on_approve(
                token_id,
                owner_id,
                approval_id,
                msg,
                &account_id,
                NO_DEPOSIT,
                env::prepaid_gas() - GAS_FOR_NFT_APPROVE,
            ))
        } else {
            None
        }
    }

    fn nft_revoke(&mut self, token_id: TokenId, account_id: ValidAccountId) {
        assert_one_yocto();
        if self.approvals_by_id.is_none() {
            env::panic(b"NFT does not support Approval Management");
        }

        let owner_id = self.owner_by_id.get(&token_id).expect("Token not found");
        let predecessor_account_id = env::predecessor_account_id();

        assert_eq!(&predecessor_account_id, &owner_id, "Predecessor must be token owner.");

        // if token has no approvals, do nothing
        if let Some(approved_account_ids) =
            &mut self.approvals_by_id.as_mut().unwrap().get(&token_id)
        {
            // if account_id was already not approved, do nothing
            if approved_account_ids.remove(account_id.as_ref()).is_some() {
                refund_approved_account_ids_iter(
                    predecessor_account_id,
                    [account_id.into()].iter(),
                );
                // if this was the last approval, remove the whole HashMap to save space.
                if approved_account_ids.is_empty() {
                    self.approvals_by_id.as_mut().unwrap().remove(&token_id);
                } else {
                    // otherwise, update approvals_by_id with updated HashMap
                    self.approvals_by_id.as_mut().unwrap().insert(&token_id, &approved_account_ids);
                }
            }
        }
    }

    fn nft_revoke_all(&mut self, token_id: TokenId) {
        assert_one_yocto();
        if self.approvals_by_id.is_none() {
            env::panic(b"NFT does not support Approval Management");
        }

        let owner_id = self.owner_by_id.get(&token_id).expect("Token not found");
        let predecessor_account_id = env::predecessor_account_id();

        assert_eq!(&predecessor_account_id, &owner_id, "Predecessor must be token owner.");

        // if token has no approvals, do nothing
        if let Some(approved_account_ids) =
            &mut self.approvals_by_id.as_mut().unwrap().get(&token_id)
        {
            // otherwise, refund owner for storage costs of all approvals...
            refund_approved_account_ids(predecessor_account_id, &approved_account_ids);
            // ...and remove whole HashMap of approvals
            self.approvals_by_id.as_mut().unwrap().remove(&token_id);
        }
    }

    fn nft_is_approved(
        self,
        token_id: TokenId,
        approved_account_id: ValidAccountId,
        approval_id: Option<u64>,
    ) -> bool {
        self.owner_by_id.get(&token_id).expect("Token not found");

        if self.approvals_by_id.is_none() {
            // contract does not support approval management
            return false;
        }

        let approved_account_ids = self.approvals_by_id.unwrap().get(&token_id);
        if approved_account_ids.is_none() {
            // token has no approvals
            return false;
        }

        let account_id: AccountId = approved_account_id.into();
        let actual_approval_id = approved_account_ids.as_ref().unwrap().get(&account_id);
        if actual_approval_id.is_none() {
            // account not in approvals HashMap
            return false;
        }

        if let Some(given_approval_id) = approval_id {
            &given_approval_id == actual_approval_id.unwrap()
        } else {
            // account approved, no approval_id given
            true
        }
    }
}
