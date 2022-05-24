
use near_sdk::json_types::U128;
use near_units::parse_near;
use test_case::test_case;
use workspaces::prelude::*;

#[test_case("factory_contract_high_level")]
#[test_case("factory_contract_low_level")]
#[tokio::test]
async fn test_deploy_status_message(contract_name: &str) -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let contract = worker.dev_deploy(&std::fs::read(format!("res/{}.wasm", contract_name))?).await?;

    // Needed because of 32 character minimum for TLA
    // https://docs.near.org/docs/concepts/account#top-level-accounts
    let status_id: near_sdk::AccountId = "status-top-level-account-long-name".parse().unwrap();
    let status_amt = U128::from(parse_near!("20 N"));
    let res = contract
        .call(&worker, "deploy_status_message")
        .args_json((status_id.clone(), status_amt))?
        .max_gas()
        .deposit(parse_near!("50 N"))
        .transact()
        .await?;
    assert!(res.is_success());

    let message = "hello world";
    let res = contract
        .call(&worker, "complex_call")
        .args_json((status_id, message))?
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());
    let value = res.json::<String>()?;
    assert_eq!(message, value.trim_matches(|c| c == '"'));

    Ok(())
}
