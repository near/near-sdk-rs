use near_sdk::{env, near};

#[derive(Default)]
#[near(contract_state)]
pub struct TestContract {
    value: u32,
}

#[near]
impl TestContract {
    #[init]
    pub fn new() -> Self {
        Self { value: 0 }
    }

    /// A private init method that can only be called by the contract itself.
    /// This is useful for factory patterns where a deployed contract should
    /// only be initialized by itself via a scheduled function call.
    #[init]
    #[private]
    pub fn new_private(value: u32) -> Self {
        Self { value }
    }

    #[init(ignore_state)]
    pub fn migrate_state() -> Self {
        #[near]
        struct OldContract {
            // ...
        }

        let _old_contract: OldContract = env::state_read().expect("Old state doesn't exist");

        Self { value: 0 }
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }

    pub fn test_panic_macro(&mut self) {
        panic!("PANIC!");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_abi::AbiRoot;
    use near_sdk::serde_json;

    async fn build_contract(path: &str) -> anyhow::Result<Vec<u8>> {
        use near_workspaces::cargo_near_build;
        use std::str::FromStr;
        let path = path.to_string();
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
            let manifest = cargo_near_build::camino::Utf8PathBuf::from_str(&path)
                .map_err(|e| anyhow::anyhow!("camino: {e}"))?
                .join("Cargo.toml");
            let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
                no_locked: true,
                manifest_path: Some(manifest),
                // 2.12 RC: contracts build on rustc 1.93 > the 1.86 declared in
                // Cargo.toml; cargo-near otherwise refuses wasm built with >= 1.87.
                skip_rust_version_check: true,
                ..Default::default()
            })
            .map_err(|e| anyhow::anyhow!("cargo near build: {e:?}"))?;
            Ok(std::fs::read(&artifact)?)
        })
        .await?
    }

    #[test]
    #[should_panic(expected = "PANIC!")]
    fn test_panic() {
        let mut contract = TestContract::new();
        contract.test_panic_macro();
    }

    // this only tests that contract can be built with ABI and responds to __contract_abi
    // view call
    #[tokio::test]
    async fn embedded_abi_test() -> anyhow::Result<()> {
        let wasm = build_contract("./").await?;
        let worker = near_workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        let res = contract.view("__contract_abi").await?;

        let abi_root = serde_json::from_slice::<AbiRoot>(&zstd::decode_all(&res.result[..])?)?;

        assert_eq!(abi_root.schema_version, "0.4.0");
        assert_eq!(abi_root.metadata.name, Some("test-contract".to_string()));

        Ok(())
    }

    /// Tests that a private init method cannot be called by an external account.
    /// The method should fail with "Method new_private is private".
    #[tokio::test]
    async fn private_init_cannot_be_called_by_external_account() -> anyhow::Result<()> {
        let wasm = build_contract("./").await?;
        let worker = near_workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        // Create an external account (alice)
        let alice = worker.dev_create_account().await?;

        // External account tries to call the private init method - should fail
        let res = alice
            .call(contract.id(), "new_private")
            .args_json((42u32,))
            .max_gas()
            .transact()
            .await?;

        assert!(res.is_failure());
        let failure_message = format!("{:?}", res.into_result().unwrap_err());
        assert!(
            failure_message.contains("Method new_private is private"),
            "Expected 'Method new_private is private' error, got: {}",
            failure_message
        );

        Ok(())
    }

    /// Tests that a private init method can be called by the contract itself.
    /// This simulates the contract calling its own init method (e.g., via a callback).
    #[tokio::test]
    async fn private_init_can_be_called_by_current_account() -> anyhow::Result<()> {
        let wasm = build_contract("./").await?;
        let worker = near_workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        // Contract calls its own private init method - should succeed
        let res = contract.call("new_private").args_json((42u32,)).max_gas().transact().await?;

        assert!(res.is_success(), "Private init should succeed when called by self: {:?}", res);

        // Verify the state was set correctly
        let value: u32 = contract.view("get_value").await?.json()?;
        assert_eq!(value, 42);

        Ok(())
    }
}
