use near_sdk::store::LookupMap;
use near_sdk::{env, log, near, AccountId, BorshStorageKey};

#[derive(BorshStorageKey)]
#[near]
struct RecordsKey;

#[near(contract_state)]
pub struct StatusMessage {
    records: LookupMap<AccountId, String>,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self { records: LookupMap::new(RecordsKey) }
    }
}

#[near]
impl StatusMessage {
    #[payable]
    pub fn set_status(&mut self, message: String) {
        let account_id = env::signer_account_id();
        log!("{} set_status with message {}", account_id, message);
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: AccountId) -> Option<&String> {
        log!("get_status for account_id {}", account_id);
        self.records.get(&account_id)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_logs, VMContextBuilder};
    use near_sdk::{testing_env, VMContext};
    use near_sdk::serde_json;
    use near_abi::AbiRoot;

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new()
            .signer_account_id("bob_near".parse().unwrap())
            .is_view(is_view)
            .build()
    }

    #[test]
    fn set_get_message() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = StatusMessage::default();
        contract.set_status("hello".to_string());
        // Flush the pending changes to avoid panic in the view method below due to the pending non-committed changes to the `store::LookupMap`:
        // HostError(ProhibitedInView { method_name: "storage_write" })
        contract.records.flush();
        assert_eq!(get_logs(), vec!["bob_near set_status with message hello"]);

        let context = get_context(true);
        testing_env!(context);
        assert_eq!("hello", contract.get_status("bob_near".parse().unwrap()).unwrap());
        assert_eq!(get_logs(), vec!["get_status for account_id bob_near"]);
    }

    #[test]
    fn get_nonexistent_message() {
        let context = get_context(true);
        testing_env!(context);
        let contract = StatusMessage::default();
        assert_eq!(None, contract.get_status("francis.near".parse().unwrap()));
        assert_eq!(get_logs(), vec!["get_status for account_id francis.near"])
    }

    // this only tests that contract can be built with ABI and responds to __contract_abi
    // view call
    #[tokio::test]
    async fn embedded_abi_test() -> anyhow::Result<()> {
        let wasm = near_workspaces::compile_project("./").await?;
        let worker = near_workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        let res = contract.view("__contract_abi").await?;

        let abi_root =
            serde_json::from_slice::<AbiRoot>(&zstd::decode_all(&res.result[..])?)?;

        assert_eq!(abi_root.schema_version, "0.4.0");
        assert_eq!(abi_root.metadata.name, Some("status-message".to_string()));

        Ok(())
    }
}
