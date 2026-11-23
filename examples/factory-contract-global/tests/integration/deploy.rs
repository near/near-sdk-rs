use near_kit::json_types::Base64VecU8;
use near_kit::rpc::Finality;
use near_kit::transaction::FunctionCall;
use near_kit::{CryptoHash, Gas};
use serde_json::json;

use crate::shared::{GLOBAL_STORAGE_COST_PER_BYTE, build_contract, setup};

#[tokio::test]
async fn test_deploy_global_contract() -> anyhow::Result<()> {
    let (_node, alice, factory_id) = setup().await?;
    let code = build_contract("../status-message").await?;

    // The factory deploys the status-message code as a global contract identified by its
    // code hash. The attached deposit pays for storing the code.
    alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("deploy_global_contract")
                .args(json!({
                    "name": "status_message",
                    "code": Base64VecU8::from(code.clone()),
                }))
                .gas(Gas::from_tgas(20))
                .deposit(GLOBAL_STORAGE_COST_PER_BYTE.saturating_mul(code.len() as u128)),
        )
        .await?
        .result()?;

    // The factory records the code hash under the given name.
    let code_hash = CryptoHash::hash(&code);

    let stored_hash: Option<CryptoHash> = alice
        .near()
        .view(&factory_id, "get_global_contract_hash")
        .args(json!({ "name": "status_message" }))
        .at(Finality::Optimistic)
        .await?;

    assert_eq!(stored_hash, Some(code_hash));

    let registered: Vec<(String, CryptoHash)> = alice
        .near()
        .view(&factory_id, "get_global_contracts_registered_by_code_hash")
        .at(Finality::Optimistic)
        .await?;

    assert_eq!(registered, [("status_message".to_string(), code_hash)]);

    Ok(())
}
