use near_workspaces::types::{AccountId, NearToken};

/// Test basic global contract deployment functionality
#[tokio::test]
async fn test_deploy_global_contract() -> anyhow::Result<()> {
    // Initialize the sandbox environment with specific commit that includes global contract support
    println!("Initializing worker");
    let worker =
        near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e")
            .await?;

    println!("Deploying global contract");

    // Compile and deploy the factory contract
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // Compile status message contract to use as global contract
    let status_code = near_workspaces::compile_project("../status-message").await?;

    // Deploy a global contract
    let global_account_id: AccountId = format!("global.{}", factory_contract.id()).parse()?;
    let deploy_amount = NearToken::from_near(20);

    let res = factory_contract
        .call("deploy_global_contract")
        .args_json(("status_message", status_code.clone(), &global_account_id, deploy_amount))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    println!("Deployed global contract: {:?}", res);

    assert!(res.is_success(), "Failed to deploy global contract: {:?}", res);

    // Verify the global contract was recorded
    let stored_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("status_message",))
        .view()
        .await?
        .json::<Option<Vec<u8>>>()?;

    assert!(stored_hash.is_some(), "Global contract hash should be stored");

    // Verify we can list the deployed global contracts
    let contracts_list = factory_contract.call("list_global_contracts").view().await?.json::<Vec<(
        String,
        Vec<u8>,
        AccountId,
    )>>()?;

    assert_eq!(contracts_list.len(), 1);
    assert_eq!(contracts_list[0].0, "status_message");

    Ok(())
}

/// Test using a global contract by hash
#[tokio::test]
async fn test_use_global_contract_by_hash() -> anyhow::Result<()> {
    let worker =
        near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e")
            .await?;
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // First deploy a global contract
    let status_code = near_workspaces::compile_project("../status-message").await?;
    let global_account_id: AccountId = format!("global.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("deploy_global_contract")
        .args_json((
            "status_message",
            status_code.clone(),
            &global_account_id,
            NearToken::from_near(20),
        ))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;

    assert!(res.is_success());

    // Get the hash of the deployed global contract
    let stored_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("status_message",))
        .view()
        .await?
        .json::<Option<Vec<u8>>>()?
        .expect("Should have stored hash");

    // Now use the global contract by hash
    let user_account_id: AccountId = format!("user.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("use_global_contract_by_hash")
        .args_json((stored_hash, &user_account_id, NearToken::from_near(10)))
        .max_gas()
        .deposit(NearToken::from_near(30))
        .transact()
        .await?;

    assert!(res.is_success(), "Failed to use global contract by hash: {:?}", res.outcome());

    Ok(())
}

/// Test using a global contract by account ID
#[tokio::test]
async fn test_use_global_contract_by_account() -> anyhow::Result<()> {
    let worker =
        near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e")
            .await?;
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // First deploy a global contract
    let status_code = near_workspaces::compile_project("../status-message").await?;
    let global_account_id: AccountId = format!("global.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("deploy_global_contract_by_account_id")
        .args_json((
            "status_message",
            status_code.clone(),
            &global_account_id,
            NearToken::from_near(20),
        ))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;

    assert!(res.is_success(), "Failed to deploy global contract by account: {:?}", res);

    // Now use the global contract by account ID
    let user_account_id: AccountId = format!("user.{}", factory_contract.id()).parse()?;

    let res = factory_contract
        .call("use_global_contract_by_account")
        .args_json((&global_account_id, &user_account_id, NearToken::from_near(10)))
        .max_gas()
        .deposit(NearToken::from_near(30))
        .transact()
        .await?;

    assert!(res.is_success(), "Failed to use global contract by account: {:?}", res);

    Ok(())
}

/// Test error cases and edge conditions
#[tokio::test]
async fn test_global_contract_edge_cases() -> anyhow::Result<()> {
    let worker =
        near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e")
            .await?;
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // Test getting hash for non-existent contract
    let non_existent_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("non_existent",))
        .view()
        .await?
        .json::<Option<Vec<u8>>>()?;

    assert!(non_existent_hash.is_none(), "Should return None for non-existent contract");

    // Test listing contracts when none are deployed
    let empty_list = factory_contract.call("list_global_contracts").view().await?.json::<Vec<(
        String,
        Vec<u8>,
        AccountId,
    )>>()?;

    assert!(empty_list.is_empty(), "Should return empty list when no contracts deployed");

    Ok(())
}
