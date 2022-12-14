use near_contract_standards::multi_token::{approval::MultiTokenApprovalReceiver, token::TokenId};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::U128,
    log, near_bindgen, AccountId, PanicOnDefault, PromiseOrValue,
};

pub const ON_MT_TOKEN_APPROVE_MSG: &str = "on_multi_token_approve";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }
}

#[near_bindgen]
impl MultiTokenApprovalReceiver for Contract {
    fn mt_on_approve(
        &mut self,
        tokens: Vec<TokenId>,
        amounts: Vec<U128>,
        owner_id: AccountId,
        approval_ids: Vec<u64>,
        msg: String,
    ) -> PromiseOrValue<String> {
        log!(
            "Tokens: {:?} Amounts: {:?} Owner: {}, approval_ids: {:?}",
            tokens,
            amounts,
            owner_id,
            approval_ids
        );
        log!(&msg);

        PromiseOrValue::Value(ON_MT_TOKEN_APPROVE_MSG.to_string())
    }
}
