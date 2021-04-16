use crate::non_fungible_token::approval::NonFungibleTokenApproval;
use crate::non_fungible_token::core_impl::NonFungibleToken;
use crate::non_fungible_token::token::TokenId;
use crate::non_fungible_token::utils::{bytes_for_approved_account_id, refund_deposit};
use near_sdk::json_types::ValidAccountId;
use near_sdk::{assert_at_least_one_yocto, env, ext_contract, AccountId, Balance, Gas, Promise};
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

        assert_eq!(
            &env::predecessor_account_id(),
            &owner_id,
            "Predecessor must be the token owner."
        );

        let approvals_by_id = self.approvals_by_id.as_mut().unwrap();

        let approved_account_ids =
            &mut approvals_by_id.get(&token_id).unwrap_or_else(|| HashMap::new());

        let account_id: AccountId = account_id.into();

        let approval_id: u64 =
            self.next_approval_id_by_id.as_ref().unwrap().get(&token_id).unwrap_or_else(|| 1u64);

        self.next_approval_id_by_id.as_mut().unwrap().insert(&token_id, &(approval_id + 1));

        let old_approval_id = approved_account_ids.insert(account_id.clone(), approval_id);

        approvals_by_id.insert(&token_id, &approved_account_ids);

        let is_new_approval = old_approval_id.is_none();

        let storage_used =
            if is_new_approval { bytes_for_approved_account_id(&account_id) } else { 0 };

        refund_deposit(storage_used);

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

    fn nft_revoke(&mut self, token_id: TokenId, account_id: ValidAccountId) {}

    fn nft_revoke_all(&mut self, token_id: TokenId) {}

    fn nft_is_approved(
        self,
        token_id: TokenId,
        approved_account_id: ValidAccountId,
        approval_id: Option<u64>,
    ) -> bool {
        false
    }
}
