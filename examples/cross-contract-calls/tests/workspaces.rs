use test_case::test_case;

#[test_case("./high-level")]
#[test_case("./low-level")]
#[tokio::test]
async fn test_factorial(contract_path: &str) -> anyhow::Result<()> {
    let wasm = near_workspaces::compile_project(contract_path).await?;
    let worker = near_workspaces::sandbox().await?;
    let contract =
        worker.dev_deploy(&wasm).await?;

    let res = contract.call("factorial").args_json((1,)).max_gas().transact().await?;
    assert!(res.is_success());

    let n = 10;
    let res = contract.call("factorial").args_json((n,)).max_gas().transact().await?;
    assert!(res.is_success());
    assert_eq!(res.json::<u32>()?, (1..n + 1).product::<u32>());

    Ok(())
}
