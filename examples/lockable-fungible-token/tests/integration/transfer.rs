use near_kit::json_types::U128;
use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas};
use serde_json::json;

use crate::shared::{TOTAL_SUPPLY, failures, setup, unlocked_balance};

/// `transfer` that sends `amount` of the caller's tokens to `new_owner_id`.
fn transfer(new_owner_id: &AccountId, amount: u128) -> FunctionCall {
    FunctionCall::new("transfer")
        .args(json!({
            "new_owner_id": new_owner_id,
            "amount": U128(amount),
        }))
        .gas(Gas::from_tgas(2))
}

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let (_node, alice, bob, contract_id) = setup().await?;

    alice.transaction(&contract_id).add_action(transfer(bob.account_id(), 100)).await?.result()?;

    let near = alice.near();

    assert_eq!(unlocked_balance(near, &contract_id, alice.account_id()).await?, TOTAL_SUPPLY - 100);
    assert_eq!(unlocked_balance(near, &contract_id, bob.account_id()).await?, 100);

    Ok(())
}

#[tokio::test]
async fn test_fail_transfer() -> anyhow::Result<()> {
    let (_node, alice, bob, contract_id) = setup().await?;

    // Each case is how many tokens Alice sends to Bob and why it fails
    let cases = [(0, "Can't transfer 0 tokens"), (TOTAL_SUPPLY + 1, "Not enough unlocked balance")];

    for (amount, error) in cases {
        let tx_details =
            alice.transaction(&contract_id).add_action(transfer(bob.account_id(), amount)).await?;

        assert_eq!(failures(&tx_details), [error], "transfer {amount}");
    }

    Ok(())
}
