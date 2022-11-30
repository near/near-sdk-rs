use serde_json::json;

const STATUS_MSG_WASM_FILEPATH: &str = "../../target/res/status_message_riff.wasm";

#[tokio::test]
async fn set_and_get_status() {
    let worker = workspaces::sandbox().await.unwrap();
    let wasm = std::fs::read(STATUS_MSG_WASM_FILEPATH).unwrap();
    let contract = worker.dev_deploy(&wasm).await.unwrap();

    let outcome = contract
        .call("set_status")
        .args_json(json!({
            "message": "hello_world",
        }))
        .transact()
        .await
        .unwrap();
    println!("set_status: {:#?}", outcome);

    let result: String = contract
        .view(
            "get_status",
            json!({
                "account_id": contract.id(),
            })
            .to_string()
            .as_str()
            .as_bytes()
            .to_vec(),
        )
        .await
        .unwrap()
        .json()
        .unwrap();

    assert_eq!(result, "hello_world");

    println!("status: {:?}", result);

    // Ok(())
}
