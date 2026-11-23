use near_kit::json_types::Base64VecU8;
use near_kit::rpc::Finality;
use near_kit::transaction::FunctionCall;
use near_kit::{CryptoHash, Gas, NearToken};
use serde_json::json;

use crate::shared::{GLOBAL_STORAGE_COST_PER_BYTE, build_contract, setup};

#[tokio::test]
async fn test_use_global_contract_by_hash() -> anyhow::Result<()> {
    let (_node, alice, factory_id) = setup().await?;
    let code = build_contract("../status-message").await?;

    // The factory deploys the status-message code as a global contract identified by its
    // code hash.
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

    // The factory creates `user.factory.alice`, which runs the global contract found by its
    // code hash instead of storing its own copy of the code.
    let user_id = factory_id.sub_account("user")?;

    alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("use_global_contract_by_hash")
                .args(json!({
                    "code_hash": CryptoHash::hash(&code),
                    "account_id": user_id,
                }))
                .gas(Gas::from_tgas(12))
                .deposit(NearToken::from_millinear(1)),
        )
        .await?
        .result()?;

    // The new account answers status-message calls.
    let status: Option<String> = alice
        .near()
        .view(&user_id, "get_status")
        .args(json!({ "account_id": alice.account_id() }))
        .at(Finality::Optimistic)
        .await?;

    assert_eq!(status, None);

    Ok(())
}

#[tokio::test]
async fn test_use_global_contract_by_account() -> anyhow::Result<()> {
    let (_node, alice, factory_id) = setup().await?;
    let code = build_contract("../status-message").await?;

    // The factory creates `global.factory.alice` and deploys the status-message code there as
    // a global contract identified by that account.
    let global_id = factory_id.sub_account("global")?;

    alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("deploy_global_contract_by_account_id")
                .args(json!({
                    "name": "status_message",
                    "code": Base64VecU8::from(code.clone()),
                    "account_id": global_id,
                }))
                .gas(Gas::from_tgas(30))
                .deposit(GLOBAL_STORAGE_COST_PER_BYTE.saturating_mul(code.len() as u128)),
        )
        .await?
        .result()?;

    // The factory creates `user.factory.alice`, which runs the global contract published by
    // `global.factory.alice`.
    let user_id = factory_id.sub_account("user")?;

    alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("use_global_contract_by_account")
                .args(json!({
                    "deployer_account_id": global_id,
                    "account_id": user_id,
                }))
                .gas(Gas::from_tgas(12))
                .deposit(NearToken::from_millinear(1)),
        )
        .await?
        .result()?;

    // The new account answers status-message calls.
    let status: Option<String> = alice
        .near()
        .view(&user_id, "get_status")
        .args(json!({ "account_id": alice.account_id() }))
        .at(Finality::Optimistic)
        .await?;

    assert_eq!(status, None);

    Ok(())
}
