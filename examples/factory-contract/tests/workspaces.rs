use near_workspaces::types::{AccountId, NearToken};
use test_case::test_case;

#[test_case("./high-level")]
#[test_case("./low-level")]
#[tokio::test]
async fn test_deploy_status_message(contract_path: &str) -> anyhow::Result<()> {
    let wasm = near_workspaces::compile_project(contract_path).await?;
    let worker = near_workspaces::sandbox().await?;
    let contract =
        worker.dev_deploy(&wasm).await?;

    let status_id: AccountId = format!("status.{}", contract.id()).parse()?;
    let status_amt = NearToken::from_near(20);
    let res = contract
        .call("deploy_status_message")
        .args_json((&status_id, status_amt))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    assert!(res.is_success());

    let message = "hello world from factory";
    let res =
        contract.call("complex_call").args_json((status_id, message)).max_gas().transact().await?;
    assert!(res.is_success());
    let value = res.json::<String>()?;
    assert_eq!(message, value.trim_matches(|c| c == '"'));

    Ok(())
}
