use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::PromiseError;
use near_sdk::{env, ext_contract, json_types::U128, near_bindgen, AccountId, Promise};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct FactoryContract {}

// One can provide a name, e.g. `ext` to use for generated methods.
#[ext_contract(ext)]
pub trait ExtFactoryContract {
    fn get_result(
        &self,
        account_id: AccountId,
        #[callback_result] set_status_result: Result<(), PromiseError>,
    ) -> Option<String>;
}

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
        ext_status_message::set_status(message, account_id.clone(), 0, prepaid_gas / 3)
            .then(ext::get_result(account_id, env::current_account_id(), 0, prepaid_gas / 3))
    }

    #[handle_result]
    pub fn get_result(
        &self,
        account_id: AccountId,
        #[callback_result] set_status_result: Result<(), PromiseError>,
    ) -> Result<Promise, &'static str> {
        let prepaid_gas = env::prepaid_gas();
        match set_status_result {
            Ok(_) => Ok(ext_status_message::get_status(
                env::signer_account_id(),
                account_id,
                0,
                prepaid_gas / 2,
            )),
            Err(_) => Err("Failed to set status"),
        }
    }
}
