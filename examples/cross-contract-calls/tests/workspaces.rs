
use test_case::test_case;
use workspaces::prelude::*;

#[test_case("cross_contract_high_level")]
#[test_case("cross_contract_low_level")]
#[tokio::test]
async fn test_factorial(contract_name: &str) -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let contract = worker.dev_deploy(&std::fs::read(format!("res/{}.wasm", contract_name))?).await?;

    let res = contract
        .call(&worker, "factorial")
        .args_json((1,))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(res.is_success());

    let n = 10;
    let res = contract
        .call(&worker, "factorial")
        .args_json((n,))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(res.is_success());
    assert_eq!(res.json::<u32>()?, (1..n + 1).product::<u32>());

    Ok(())
}
