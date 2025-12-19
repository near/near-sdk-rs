use near_sdk::serde_json;
use near_workspaces::Contract;
use near_workspaces::cargo_near_build;
use serde_json::json;
use rstest::{fixture, rstest};
use std::str::FromStr;

async fn get_value(contract: &Contract) -> anyhow::Result<u64> {
    let get_value: serde_json::Value =
        contract.call("get_value").args_json(json!({})).view().await?.json()?;

    println!("get_value: {:?}", get_value);

    get_value.as_u64().ok_or_else(|| anyhow::anyhow!("get_value is not a u64"))
}

async fn check_call(
    contract: &Contract,
    method: &str,
    is_error: bool,
    expected_value: u64,
    expected_error: Option<String>,
) {
    let res = contract
        .call(method)
        .args_json(json!({ "is_error": is_error }))
        .max_gas()
        .transact()
        .await
        .unwrap();
    if is_error {
        assert!(res.is_failure());
        if let Some(expected_error) = expected_error {
            let string_error =
                format!("{:?}", res.failures()[0].clone().into_result().unwrap_err());
            assert_eq!(string_error, expected_error);
        }
    } else {
        assert!(res.is_success());
    }
    assert_eq!(get_value(&contract).await.unwrap(), expected_value);
}

fn build_contract(path: &str, contract_name: &str) -> Vec<u8> {
    let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
        manifest_path: Some(
            cargo_near_build::camino::Utf8PathBuf::from_str(path).expect("camino PathBuf from str"),
        ),
        ..Default::default()
    })
    .expect(&format!("building `{}` contract for tests", contract_name));

    let contract_wasm = std::fs::read(&artifact)
        .map_err(|err| format!("accessing {} to read wasm contents: {}", artifact, err))
        .expect("std::fs::read");
    contract_wasm
}

#[fixture]
#[once]
fn error_handling_contract_wasm() -> Vec<u8> {
    build_contract("./Cargo.toml", "error-handling")
}

#[rstest]
#[tokio::test]
async fn test_error_handling(
    error_handling_contract_wasm: &Vec<u8>,
) -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = worker.dev_deploy(error_handling_contract_wasm).await?;

    check_call(&contract, "inc_handle_result", false, 1, None).await;
    check_call(&contract, "inc_persist_on_err", false, 2, None).await;
    check_call(&contract, "inc_just_result", false, 3, None).await;
    check_call(&contract, "inc_just_simple", false, 4, None).await;
    check_call(&contract, "inc_base", false, 5, None).await;
    check_call(&contract, "inc_handle_result", true, 5, None).await;
    check_call(&contract, "inc_persist_on_err", true, 6, Some("Error { repr: Custom { kind: Execution, error: ActionError(ActionError { index: Some(0), kind: FunctionCallError(ExecutionError(\"Smart contract panicked: {\\\"error\\\":{\\\"name\\\":\\\"CUSTOM_CONTRACT_ERROR\\\",\\\"cause\\\":{\\\"name\\\":\\\"error_handling::MyErrorEnum\\\",\\\"info\\\":\\\"X\\\"}}}\")) }) } }".to_string())).await;
    check_call(&contract, "inc_just_result", true, 6, Some("Error { repr: Custom { kind: Execution, error: ActionError(ActionError { index: Some(0), kind: FunctionCallError(ExecutionError(\"Smart contract panicked: {\\\"error\\\":{\\\"name\\\":\\\"SDK_CONTRACT_ERROR\\\",\\\"cause\\\":{\\\"name\\\":\\\"error_handling::MyErrorStruct\\\",\\\"info\\\":{\\\"x\\\":5}}}}\")) }) } }".to_string())).await;
    check_call(&contract, "inc_base", true, 6, Some("Error { repr: Custom { kind: Execution, error: ActionError(ActionError { index: Some(0), kind: FunctionCallError(ExecutionError(\"Smart contract panicked: {\\\"error\\\":{\\\"name\\\":\\\"SDK_CONTRACT_ERROR\\\",\\\"cause\\\":{\\\"name\\\":\\\"error_handling::MyErrorStruct\\\",\\\"info\\\":{\\\"x\\\":5}}}}\")) }) } }".to_string())).await;
    check_call(&contract, "inc_just_simple", true, 6, None).await;

    Ok(())
}
