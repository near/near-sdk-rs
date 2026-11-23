use near_kit::json_types::U128;
use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas};
use serde_json::json;

use crate::shared::{allowance, failures, setup};

/// `set_allowance` that lets `escrow_account_id` spend `amount` of the caller's tokens.
fn set_allowance(escrow_account_id: &AccountId, amount: u128) -> FunctionCall {
    FunctionCall::new("set_allowance")
        .args(json!({
            "escrow_account_id": escrow_account_id,
            "allowance": U128(amount),
        }))
        .gas(Gas::from_tgas(1))
}

#[tokio::test]
async fn test_set_allowance() -> anyhow::Result<()> {
    let (_node, alice, bob, contract_id) = setup().await?;

    alice
        .transaction(&contract_id)
        .add_action(set_allowance(bob.account_id(), 100))
        .await?
        .result()?;

    let near = alice.near();

    assert_eq!(allowance(near, &contract_id, alice.account_id(), alice.account_id()).await?, 0);
    assert_eq!(allowance(near, &contract_id, alice.account_id(), bob.account_id()).await?, 100);

    Ok(())
}

#[tokio::test]
async fn test_fail_set_allowance_self() -> anyhow::Result<()> {
    let (_node, alice, _, contract_id) = setup().await?;

    let tx_details =
        alice.transaction(&contract_id).add_action(set_allowance(alice.account_id(), 100)).await?;

    assert_eq!(failures(&tx_details), ["Can't set allowance for yourself"]);

    Ok(())
}
