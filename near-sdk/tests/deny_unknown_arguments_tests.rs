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
    let outcome = user_account
        .call(contract.id(), "new")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "starting_value": 0,
            "unknown_field": 1,
        }))
        .transact()
        .await?;
    assert!(outcome.is_failure());
    // Now all should be fine
    let outcome = user_account
        .call(contract.id(), "new")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "starting_value": 0,
        }))
        .transact()
        .await?;
    assert!(outcome.is_success());

    // mut method check
    let outcome = user_account
        .call(contract.id(), "inc")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
            "unknown_field": 1,
        }))
        .transact()
        .await?;
    assert!(outcome.is_failure());
    let outcome = user_account
        .call(contract.id(), "inc")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
        }))
        .transact()
        .await?;
    assert!(outcome.is_success());

    // view method check
    let outcome = user_account
        .call(contract.id(), "inc_view")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
            "unknown_field": 1,
        }))
        .transact()
        .await?;
    assert!(outcome.is_failure());
    let outcome = user_account
        .call(contract.id(), "inc_view")
        .gas(Gas::from_tgas(10))
        .args_json(json!({
            "by": 3,
        }))
        .transact()
        .await?;
    assert!(outcome.is_success());
    Ok(())
}
