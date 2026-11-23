use near_kit::protocol::ActionErrorKind;
use near_kit::rpc::{Finality, OutcomeError};
use near_kit::transaction::FunctionCall;
use near_kit::{CryptoHash, Gas, NearToken};
use serde_json::json;

use crate::shared::setup;

/// Whether `error` is a failed action that referenced a global contract that doesn't exist.
fn is_missing_global_contract(error: &OutcomeError) -> bool {
    matches!(
        error,
        OutcomeError::ActionFailed(action)
            if matches!(action.kind, ActionErrorKind::GlobalContractDoesNotExist { .. })
    )
}

#[tokio::test]
async fn test_global_contract_edge_cases() -> anyhow::Result<()> {
    let (_node, alice, factory_id) = setup().await?;

    // Nothing is registered before a global contract is deployed.
    let stored_hash: Option<CryptoHash> = alice
        .near()
        .view(&factory_id, "get_global_contract_hash")
        .args(json!({ "name": "non_existent" }))
        .at(Finality::Optimistic)
        .await?;

    assert_eq!(stored_hash, None);

    let registered: Vec<(String, CryptoHash)> = alice
        .near()
        .view(&factory_id, "get_global_contracts_registered_by_code_hash")
        .at(Finality::Optimistic)
        .await?;

    assert!(registered.is_empty());

    // Using a global contract that was never deployed fails, whether it is looked up by
    // code hash or by deployer account.
    let user_id = factory_id.sub_account("user")?;

    let error = alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("use_global_contract_by_hash")
                .args(json!({
                    "code_hash": CryptoHash::ZERO,
                    "account_id": user_id,
                }))
                .gas(Gas::from_tgas(12))
                .deposit(NearToken::from_millinear(1)),
        )
        .await?
        .result()
        .expect_err("using a missing global contract by hash should fail");

    assert!(is_missing_global_contract(&error), "{error}");

    let missing_deployer_id = factory_id.sub_account("non-existent-global")?;

    let error = alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("use_global_contract_by_account")
                .args(json!({
                    "deployer_account_id": missing_deployer_id,
                    "account_id": user_id,
                }))
                .gas(Gas::from_tgas(12))
                .deposit(NearToken::from_millinear(1)),
        )
        .await?
        .result()
        .expect_err("using a missing global contract by account should fail");

    assert!(is_missing_global_contract(&error), "{error}");

    Ok(())
}
