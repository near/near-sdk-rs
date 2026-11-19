use near_kit::json_types::U128;
use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas};
use serde_json::json;

use crate::shared::{TOTAL_SUPPLY, allowance, failures, locked_balance, setup, unlocked_balance};

/// `lock` that locks `amount` of `owner_id`'s tokens for the caller.
fn lock(owner_id: &AccountId, amount: u128) -> FunctionCall {
    FunctionCall::new("lock")
        .args(json!({
            "owner_id": owner_id,
            "lock_amount": U128(amount),
        }))
        .gas(Gas::from_tgas(1))
}

/// `unlock` that gives `amount` of the tokens the caller locked back to `owner_id`.
fn unlock(owner_id: &AccountId, amount: u128) -> FunctionCall {
    FunctionCall::new("unlock")
        .args(json!({
            "owner_id": owner_id,
            "unlock_amount": U128(amount),
        }))
        .gas(Gas::from_tgas(1))
}

#[tokio::test]
async fn test_lock_owner() -> anyhow::Result<()> {
    let (_node, alice, _, contract_id) = setup().await?;
    let alice_id = alice.account_id();

    alice.transaction(&contract_id).add_action(lock(alice_id, 100)).await?.result()?;

    let near = alice.near();

    assert_eq!(unlocked_balance(near, &contract_id, alice_id).await?, TOTAL_SUPPLY - 100);
    assert_eq!(allowance(near, &contract_id, alice_id, alice_id).await?, 0);
    assert_eq!(locked_balance(near, &contract_id, alice_id, alice_id).await?, 100);

    Ok(())
}

#[tokio::test]
async fn test_fail_lock() -> anyhow::Result<()> {
    let (_node, alice, bob, contract_id) = setup().await?;
    let alice_id = alice.account_id();

    // Each case is who locks Alice's tokens, how many and why it fails
    let cases = [
        (&alice, 0, "Can't lock 0 tokens"),
        (&alice, TOTAL_SUPPLY + 1, "Not enough unlocked balance"),
        (&bob, 10, "Not enough allowance"),
    ];

    for (escrow, amount, error) in cases {
        let tx_details =
            escrow.transaction(&contract_id).add_action(lock(alice_id, amount)).await?;

        assert_eq!(failures(&tx_details), [error], "{} locks {amount}", escrow.account_id());
    }

    Ok(())
}

#[tokio::test]
async fn test_unlock_owner() -> anyhow::Result<()> {
    let (_node, alice, _, contract_id) = setup().await?;
    let alice_id = alice.account_id();

    alice.transaction(&contract_id).add_action(lock(alice_id, 100)).await?.result()?;
    alice.transaction(&contract_id).add_action(unlock(alice_id, 100)).await?.result()?;

    let near = alice.near();

    assert_eq!(unlocked_balance(near, &contract_id, alice_id).await?, TOTAL_SUPPLY);
    assert_eq!(allowance(near, &contract_id, alice_id, alice_id).await?, 0);
    assert_eq!(locked_balance(near, &contract_id, alice_id, alice_id).await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_fail_unlock() -> anyhow::Result<()> {
    let (_node, alice, _, contract_id) = setup().await?;
    let alice_id = alice.account_id();

    // Each case is how many tokens Alice unlocks and why it fails
    let cases = [(0, "Can't unlock 0 tokens"), (1, "Not enough locked tokens")];

    for (amount, error) in cases {
        let tx_details =
            alice.transaction(&contract_id).add_action(unlock(alice_id, amount)).await?;

        assert_eq!(failures(&tx_details), [error], "unlock {amount}");
    }

    Ok(())
}
