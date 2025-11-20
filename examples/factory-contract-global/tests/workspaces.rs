use near_sdk::json_types::{Base58CryptoHash, Base64VecU8};
use near_workspaces::types::{AccountId, NearToken};

const STORAGE_DEPOSIT_PER_BYTE: NearToken = NearToken::from_near(1).saturating_div(100_000);
const GLOBAL_STORAGE_COST_PER_BYTE: NearToken = STORAGE_DEPOSIT_PER_BYTE.saturating_mul(10);

/// Test basic global contract deployment functionality
#[tokio::test]
async fn test_deploy_global_contract() -> anyhow::Result<()> {
    // Initialize the sandbox environment with specific commit that includes global contract support
    println!("Initializing worker");
    let worker = near_workspaces::sandbox_with_version("2.7.0").await?;

    println!("Deploying global contract");

    // Compile and deploy the factory contract
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // Compile status message contract to use as global contract
    let status_code = near_workspaces::compile_project("../status-message").await?;

    // Deploy a global contract
    let global_account_id: AccountId = format!("global.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("deploy_global_contract")
        .args_json(("status_message", Base64VecU8::from(status_code.clone())))
        .max_gas()
        .deposit(GLOBAL_STORAGE_COST_PER_BYTE.saturating_mul(status_code.len().try_into().unwrap()))
        .transact()
        .await?;
    println!("Deployed global contract: {res:?}");

    assert!(res.is_success(), "Failed to deploy global contract: {res:?}");

    // Verify the global contract was recorded
    let stored_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("status_message",))
        .view()
        .await?
        .json::<Option<Base58CryptoHash>>()?;

    assert!(stored_hash.is_some(), "Global contract hash should be stored");

    // Verify we can list the deployed global contracts
    let contracts_list = factory_contract
        .call("get_global_contracts_registered_by_code_hash")
        .view()
        .await?
        .json::<Vec<(String, Base58CryptoHash)>>()?;

    assert_eq!(contracts_list.len(), 1);
    assert_eq!(contracts_list[0].0, "status_message");

    Ok(())
}

/// Test using a global contract by hash
#[tokio::test]
async fn test_use_global_contract_by_hash() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox_with_version("2.7.0").await?;
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // First deploy a global contract
    let status_code = near_workspaces::compile_project("../status-message").await?;
    let global_account_id: AccountId = format!("global.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("deploy_global_contract")
        .args_json(("status_message", Base64VecU8::from(status_code.clone())))
        .max_gas()
        .deposit(GLOBAL_STORAGE_COST_PER_BYTE.saturating_mul(status_code.len().try_into().unwrap()))
        .transact()
        .await?;

    assert!(res.is_success());

    // Get the hash of the deployed global contract
    let stored_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("status_message",))
        .view()
        .await?
        .json::<Option<Base58CryptoHash>>()?
        .expect("Should have stored hash");

    // Now use the global contract by hash
    let user_account_id: AccountId = format!("user.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("use_global_contract_by_hash")
        .args_json((stored_hash, &user_account_id))
        .max_gas()
        .deposit(NearToken::from_millinear(1))
        .transact()
        .await?;

    assert!(res.is_success(), "Failed to use global contract by hash: {:?}", res.outcome());

    Ok(())
}

/// Test using a global contract by account ID
#[tokio::test]
async fn test_use_global_contract_by_account() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox_with_version("2.7.0").await?;
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // First deploy a global contract
    let status_code = near_workspaces::compile_project("../status-message").await?;
    let global_account_id: AccountId = format!("global.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("deploy_global_contract_by_account_id")
        .args_json(("status_message", Base64VecU8::from(status_code.clone()), &global_account_id))
        .max_gas()
        .deposit(GLOBAL_STORAGE_COST_PER_BYTE.saturating_mul(status_code.len().try_into().unwrap()))
        .transact()
        .await?;

    assert!(res.is_success(), "Failed to deploy global contract by account: {res:?}");

    // Now use the global contract by account ID
    let user_account_id: AccountId = format!("user.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("use_global_contract_by_account")
        .args_json((&global_account_id, &user_account_id))
        .max_gas()
        .deposit(NearToken::from_millinear(1))
        .transact()
        .await?;

    assert!(res.is_success(), "Failed to use global contract by account: {res:?}");

    Ok(())
}

/// Test error cases and edge conditions
#[tokio::test]
async fn test_global_contract_edge_cases() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox_with_version("2.7.0").await?;
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // Test getting hash for non-existent contract
    let non_existent_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("non_existent",))
        .view()
        .await?
        .json::<Option<Base58CryptoHash>>()?;

    assert!(non_existent_hash.is_none(), "Should return None for non-existent contract");

    // Test listing contracts when none are deployed
    let empty_list = factory_contract
        .call("get_global_contracts_registered_by_code_hash")
        .view()
        .await?
        .json::<Vec<(String, Base58CryptoHash)>>()?;

    assert!(empty_list.is_empty(), "Should return empty list when no contracts deployed");

    // Test using non-existent contract
    let user_account_id: AccountId = format!("user.{}", factory_contract.id()).parse()?;
    let res = factory_contract
        .call("use_global_contract_by_hash")
        .args_json(("11111111111111111111111111111111", &user_account_id))
        .max_gas()
        .deposit(NearToken::from_millinear(100))
        .transact()
        .await?;
    assert!(res.is_failure(), "Not failed to use global contract by hash: {res:?}");

    // Test using non-existent contract
    let global_contract_account_id: AccountId =
        format!("non-existent-global.{}", factory_contract.id()).parse()?;
    let user_account_id: AccountId = format!("user.{}", factory_contract.id()).parse()?;
    let res = factory_contract
        .call("use_global_contract_by_account")
        .args_json((&global_contract_account_id, &user_account_id))
        .max_gas()
        .deposit(NearToken::from_millinear(100))
        .transact()
        .await?;

    assert!(res.is_failure(), "Not failed to use global contract by account: {res:?}");

    Ok(())
}
