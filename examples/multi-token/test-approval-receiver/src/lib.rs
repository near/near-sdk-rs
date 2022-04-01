use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, AccountId, PanicOnDefault, PromiseOrValue,
};
use near_contract_standards::multi_token::{approval::MultiTokenApprovalReceiver, token::TokenId};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    /*
        initialization function (can only be called once).
        this initializes the contract with default data and the owner ID
        that's passed in
    */
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
        owner_id: AccountId,
        approval_ids: Vec<u64>,
        msg: String,
    ) -> PromiseOrValue<String> {
        env::log_str(
            format!("Tokens: {:?} Owner: {}, approval_ids: {:?}", tokens, owner_id, approval_ids)
                .as_str(),
        );
        env::log_str(&msg);

        PromiseOrValue::Value("yeeeeeeeeeeeeeeee".to_string())
    }
}
