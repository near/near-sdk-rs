use near_workspaces::types::{AccountId, NearToken};
use test_case::test_case;

#[test_case("factory_contract_high_level")]
#[test_case("factory_contract_low_level")]
#[tokio::test]
async fn test_deploy_status_message(contract_name: &str) -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract =
        worker.dev_deploy(&std::fs::read(format!("res/{}.wasm", contract_name))?).await?;

    // Needed because of 32 character minimum for TLA
    // https://docs.near.org/docs/concepts/account#top-level-accounts
    let status_id: AccountId = "status-top-level-account-long-name".parse()?;
    let status_amt = NearToken::from_near(20);
    let res = contract
        .call("deploy_status_message")
        .args_json((&status_id, status_amt))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    assert!(res.is_success());

    let message = "hello world";
    let res =
        contract.call("complex_call").args_json((status_id, message)).max_gas().transact().await?;
    assert!(res.is_success());
    let value = res.json::<String>()?;
    assert_eq!(message, value.trim_matches(|c| c == '"'));

    Ok(())
}
