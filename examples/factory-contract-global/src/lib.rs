use near_sdk::json_types::{Base58CryptoHash, Base64VecU8};
use near_sdk::{env, ext_contract, near, AccountId, CryptoHash, Promise, PromiseError};

#[derive(Default)]
#[near(contract_state)]
pub struct GlobalFactoryContract {
    /// Store the hash of deployed global contracts for reference
    pub deployed_global_contracts: std::collections::HashMap<String, CryptoHash>,
    /// Store account IDs that have deployed global contracts
    pub global_contract_deployers: std::collections::HashMap<String, AccountId>,
}

// Example contract interface that we'll deploy as a global contract
#[ext_contract]
pub trait ExtStatusMessage {
    fn set_status(&mut self, message: String);
    fn get_status(&self, account_id: AccountId) -> Option<String>;
}

#[near]
impl GlobalFactoryContract {
    /// Deploy a global contract with the given bytecode, identifiable by its code hash
    #[payable]
    pub fn deploy_global_contract(
        &mut self,
        name: String,
        code: Base64VecU8,
        account_id: AccountId,
    ) -> Promise {
        let code_hash = env::sha256_array(&code);
        self.deployed_global_contracts.insert(name.clone(), code_hash);

        Promise::new(account_id)
            .create_account()
            .transfer(env::attached_deposit())
            .add_full_access_key(env::signer_account_pk())
            .deploy_global_contract(code)
    }

    /// Deploy a global contract, identifiable by the predecessor's account ID
    #[payable]
    pub fn deploy_global_contract_by_account_id(
        &mut self,
        name: String,
        code: Base64VecU8,
        account_id: AccountId,
    ) -> Promise {
        self.global_contract_deployers.insert(name, account_id.clone());

        Promise::new(account_id)
            .create_account()
            .transfer(env::attached_deposit())
            .add_full_access_key(env::signer_account_pk())
            .deploy_global_contract_by_account_id(code)
    }

    /// Use an existing global contract by its code hash
    pub fn use_global_contract_by_hash(
        &self,
        code_hash: Base58CryptoHash,
        account_id: AccountId,
    ) -> Promise {
        Promise::new(account_id)
            .create_account()
            .transfer(env::attached_deposit())
            .add_full_access_key(env::signer_account_pk())
            .use_global_contract(code_hash)
    }

    /// Use an existing global contract by referencing the account that deployed it
    pub fn use_global_contract_by_account(
        &self,
        deployer_account_id: AccountId,
        account_id: AccountId,
    ) -> Promise {
        Promise::new(account_id)
            .create_account()
            .transfer(env::attached_deposit())
            .add_full_access_key(env::signer_account_pk())
            .use_global_contract_by_account_id(deployer_account_id)
    }

    /// Get the code hash of a deployed global contract by name
    pub fn get_global_contract_hash(&self, name: String) -> Option<Base58CryptoHash> {
        self.deployed_global_contracts.get(&name).cloned().map(|hash| hash.into())
    }

    /// Get the deployer account ID of a global contract by name
    pub fn get_global_contract_deployer(&self, name: String) -> Option<AccountId> {
        self.global_contract_deployers.get(&name).cloned()
    }

    /// List all deployed global contracts
    pub fn list_global_contracts(&self) -> Vec<(String, String)> {
        self.global_contract_deployers
            .iter()
            .map(|(name, account_id)| (name.clone(), account_id.to_string()))
            .chain(
                self.deployed_global_contracts
                    .iter()
                    .map(|(name, hash)| (name.clone(), (&Base58CryptoHash::from(*hash)).into())),
            )
            .collect()
    }

    /// Example of calling a status message contract that was deployed as global
    pub fn call_global_status_contract(&mut self, account_id: AccountId, message: String) {
        ext_status_message::ext(account_id).set_status(message);
    }

    /// Example of complex call using global contracts
    pub fn complex_global_call(&mut self, account_id: AccountId, message: String) -> Promise {
        // 1) call global status_message to record a message from the signer.
        // 2) call global status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        ext_status_message::ext(account_id.clone())
            .set_status(message)
            .then(Self::ext(env::current_account_id()).get_result(account_id))
    }

    #[handle_result]
    pub fn get_result(
        &self,
        account_id: AccountId,
        #[callback_result] set_status_result: Result<(), PromiseError>,
    ) -> Result<Promise, &'static str> {
        match set_status_result {
            Ok(_) => Ok(ext_status_message::ext(account_id).get_status(env::signer_account_id())),
            Err(_) => Err("Failed to set status"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};

    fn get_context(predecessor_account_id: AccountId) -> near_sdk::VMContext {
        VMContextBuilder::new()
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id)
            .build()
    }

    #[test]
    fn test_deploy_global_contract() {
        let context = get_context(accounts(1));
        testing_env!(context);

        let mut contract = GlobalFactoryContract::default();
        let code = vec![0u8; 100]; // Mock bytecode
        let account_id = accounts(2);

        contract.deploy_global_contract(
            "test_contract".to_string(),
            code.clone().into(),
            account_id,
        );

        // Check that the contract was recorded
        let stored_hash = contract.get_global_contract_hash("test_contract".to_string());
        assert!(stored_hash.is_some());

        let expected_hash = near_sdk::env::sha256_array(&code);
        assert_eq!(stored_hash.unwrap(), expected_hash.into());
    }

    #[test]
    fn test_list_global_contracts() {
        let context = get_context(accounts(1));
        testing_env!(context);

        let mut contract = GlobalFactoryContract::default();
        let code = vec![0u8; 100];
        let account_id = accounts(2);

        contract.deploy_global_contract(
            "test_contract".to_string(),
            code.clone().into(),
            account_id.clone(),
        );

        let contracts = contract.list_global_contracts();
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].0, "test_contract");
        assert_eq!(contracts[0].1, "EoFMvgbdQttJ3vLsVBcgZaWbhEGrJnpqda85qtbu7LbL");
    }
}
