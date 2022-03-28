use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, json_types::U128, log, near_bindgen, AccountId, Promise};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct FactoryContract {}

// If the `ext_contract` name is not provided explicitly, the namespace for generated methods is
// derived by applying snake case to the trait name, e.g. ext_status_message.
#[ext_contract]
pub trait ExtStatusMessage {
    fn set_status(&mut self, message: String);
    fn get_status(&self, account_id: AccountId) -> Option<String>;
}

#[near_bindgen]
impl FactoryContract {
    pub fn deploy_status_message(&self, account_id: AccountId, amount: U128) {
        Promise::new(account_id)
            .create_account()
            .transfer(amount.0)
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(
                include_bytes!("../../../status-message/res/status_message.wasm").to_vec(),
            );
    }

    pub fn simple_call(&mut self, account_id: AccountId, message: String) {
        ext_status_message::set_status(message, account_id, 0, env::prepaid_gas() / 2);
    }
    pub fn complex_call(&mut self, account_id: AccountId, message: String) -> Promise {
        // 1) call status_message to record a message from the signer.
        // 2) call status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        let prepaid_gas = env::prepaid_gas();
        log!("complex_call");
        ext_status_message::set_status(message, account_id.clone(), 0, prepaid_gas / 3).then(
            ext_status_message::get_status(
                env::signer_account_id(),
                account_id,
                0,
                prepaid_gas / 3,
            ),
        )
    }
}
