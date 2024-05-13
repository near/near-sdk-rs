use test_case::test_case;
use serde_json::json;
use near_workspaces::Contract;

async fn get_value(contract: &Contract) -> anyhow::Result<u64> {
    let get_value: serde_json::Value = contract
        .call("get_value")
        .args_json(json!({}))
        .view()
        .await?
        .json()?;

    println!("get_value: {:?}", get_value);

    get_value.as_u64().ok_or_else(|| anyhow::anyhow!("get_value is not a u64"))
}

#[tokio::test]
async fn test_factorial() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract =
        worker.dev_deploy(&std::fs::read(format!("res/{}.wasm", "error_handling"))?).await?;

    let res = contract.call("inc_handle_result").args_json(::near_sdk::serde_json::json!{{"is_error": false}}).max_gas().transact().await?;
    assert!(res.is_success());
    assert_eq!(get_value(&contract).await.unwrap(), 1);    

    let res = contract.call("inc_persist_on_e").args_json(::near_sdk::serde_json::json!{{"is_error": false}}).max_gas().transact().await?;
    assert!(res.is_success());
    assert_eq!(get_value(&contract).await.unwrap(), 2);    

    let res = contract.call("inc_just_result").args_json(::near_sdk::serde_json::json!{{"is_error": false}}).max_gas().transact().await?;
    assert!(res.is_success());
    assert_eq!(get_value(&contract).await.unwrap(), 3);    

    let res = contract.call("inc_just_simple").args_json(::near_sdk::serde_json::json!{{"is_error": false}}).max_gas().transact().await?;
    assert!(res.is_success());
    assert_eq!(get_value(&contract).await.unwrap(), 4);

    let res = contract.call("inc_handle_result").args_json(::near_sdk::serde_json::json!{{"is_error": true}}).max_gas().transact().await?;
    assert!(res.is_failure());
    assert_eq!(get_value(&contract).await.unwrap(), 4);    

    let res = contract.call("inc_persist_on_e").args_json(::near_sdk::serde_json::json!{{"is_error": true}}).max_gas().transact().await?;
    assert!(res.is_failure());
    let string_error = format!("{:?}",res.failures()[0].clone().into_result().unwrap_err());
    assert_eq!(string_error, "Error { repr: Custom { kind: Execution, error: ActionError(ActionError { index: Some(0), kind: FunctionCallError(ExecutionError(\"Smart contract panicked: {\\\"error\\\":{\\\"error_type\\\":\\\"error_handling::MyErrorEnum\\\",\\\"value\\\":\\\"X\\\"}}\")) }) } }");
    
    assert_eq!(get_value(&contract).await.unwrap(), 5);    

    let res = contract.call("inc_just_result").args_json(::near_sdk::serde_json::json!{{"is_error": true}}).max_gas().transact().await?;
    assert!(res.is_failure());
    let string_error = format!("{:?}",res.failures()[0].clone().into_result().unwrap_err());
    assert_eq!(string_error, "Error { repr: Custom { kind: Execution, error: ActionError(ActionError { index: Some(0), kind: FunctionCallError(ExecutionError(\"Smart contract panicked: {\\\"error\\\":{\\\"error_type\\\":\\\"error_handling::MyErrorStruct\\\",\\\"value\\\":{\\\"x\\\":5}}}\")) }) } }");
    assert_eq!(get_value(&contract).await.unwrap(), 5);

    let res = contract.call("inc_just_simple").args_json(::near_sdk::serde_json::json!{{"is_error": true}}).max_gas().transact().await?;
    assert!(res.is_failure());
    assert_eq!(get_value(&contract).await.unwrap(), 5);

    Ok(())
}
