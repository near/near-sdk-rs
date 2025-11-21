use near_sdk::Gas;
use serde_json::json;

#[tokio::test]
async fn test_contract_is_operational() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm =
        near_workspaces::compile_project("./tests/test-contracts/deny_unknown_arguments").await?;
    let sandbox = near_workspaces::sandbox().await?;

    // Create basic accounts and deploy main contract
    let contract = sandbox.dev_deploy(&contract_wasm).await?;
    let user_account = sandbox.dev_create_account().await?;

    // First we should get serialization error
    user_account
        .call(contract.id(), "new")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "starting_value": 0,
            "unknown_field": 1,
        }))
        .transact()
        .await?
        .into_result()
        .expect_err("Expected deserialization error due to unknown field");
    // Now all should be fine
    user_account
        .call(contract.id(), "new")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "starting_value": 0,
        }))
        .transact()
        .await?
        .into_result()?;

    // mut method check
    user_account
        .call(contract.id(), "inc")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
            "unknown_field": 1,
        }))
        .transact()
        .await?
        .into_result()
        .expect_err("Expected deserialization error due to unknown field");
    user_account
        .call(contract.id(), "inc")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
        }))
        .transact()
        .await?
        .into_result()?;

    // view method check
    user_account
        .call(contract.id(), "inc_view")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
            "unknown_field": 1,
        }))
        .transact()
        .await?
        .into_result()
        .expect_err("Expected deserialization error due to unknown field");
    user_account
        .call(contract.id(), "inc_view")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
        }))
        .transact()
        .await?
        .into_result()?;

    Ok(())
}
