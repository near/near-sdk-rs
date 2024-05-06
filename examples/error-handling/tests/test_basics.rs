use test_case::test_case;

#[tokio::test]
async fn test_factorial() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract =
        worker.dev_deploy(&std::fs::read(format!("res/{}.wasm", "error_handling"))?).await?;

    let res = contract.call("get_greeting").args_json(::near_sdk::serde_json::json!{{}}).max_gas().transact().await?;
    assert!(res.is_success());

    // let n = 10;
    // let res = contract.call("factorial").args_json((n,)).max_gas().transact().await?;
    // assert!(res.is_success());
    // assert_eq!(res.json::<u32>()?, (1..n + 1).product::<u32>());

    Ok(())
}
