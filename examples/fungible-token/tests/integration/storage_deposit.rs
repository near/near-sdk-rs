use near_kit::standards::{StorageBalance, StorageBalanceBounds};
use near_kit::transaction::FunctionCall;
use near_kit::{Gas, NearToken};
use serde_json::json;

use crate::shared::{ONE_YOCTO, STORAGE_DEPOSIT, failures, setup};

#[tokio::test]
async fn test_storage_deposit_not_enough_deposit() -> anyhow::Result<()> {
    let (_node, _, bob, ft_contract_id) = setup().await?;
    let near = bob.near();

    let bob_balance_before = near.balance(bob.account_id()).await?.total;
    let ft_balance_before = near.balance(&ft_contract_id).await?.total;

    let tx_details = bob
        .transaction(&ft_contract_id)
        .add_action(
            FunctionCall::new("storage_deposit")
                .gas(Gas::from_ggas(1500))
                .deposit(STORAGE_DEPOSIT.saturating_sub(ONE_YOCTO)),
        )
        .await?;

    assert_eq!(
        failures(&tx_details),
        ["The attached deposit is less than the minimum storage balance"]
    );

    // Bob gets the deposit back and pays only the transaction fee
    let bob_spent = bob_balance_before.saturating_sub(near.balance(bob.account_id()).await?.total);

    assert!(bob_spent > NearToken::ZERO);
    assert!(bob_spent < NearToken::from_millinear(1));

    // The token contract keeps nothing
    assert_eq!(near.balance(&ft_contract_id).await?.total, ft_balance_before);

    Ok(())
}

#[tokio::test]
async fn test_storage_deposit_minimal_deposit() -> anyhow::Result<()> {
    let (_node, _, bob, ft_contract_id) = setup().await?;

    bob.transaction(&ft_contract_id)
        .add_action(
            FunctionCall::new("storage_deposit").gas(Gas::from_ggas(1500)).deposit(STORAGE_DEPOSIT),
        )
        .await?
        .result()?;

    let storage_balance: Option<StorageBalance> = bob
        .near()
        .view(&ft_contract_id, "storage_balance_of")
        .args(json!({ "account_id": bob.account_id() }))
        .await?;

    let storage_balance = storage_balance.expect("Bob should be registered");

    assert_eq!(storage_balance.total, STORAGE_DEPOSIT);
    assert_eq!(storage_balance.available, NearToken::ZERO);

    Ok(())
}

#[tokio::test]
async fn test_storage_deposit_refunds_excessive_deposit() -> anyhow::Result<()> {
    let (_node, _, bob, ft_contract_id) = setup().await?;
    let near = bob.near();

    // Registration costs exactly `STORAGE_DEPOSIT`, no more and no less
    let bounds: StorageBalanceBounds = near.view(&ft_contract_id, "storage_balance_bounds").await?;

    assert_eq!(bounds.min, STORAGE_DEPOSIT);
    assert_eq!(bounds.max, Some(STORAGE_DEPOSIT));

    // Bob isn't registered yet, so he has no storage balance
    let storage_balance: Option<StorageBalance> = near
        .view(&ft_contract_id, "storage_balance_of")
        .args(json!({ "account_id": bob.account_id() }))
        .await?;

    assert!(storage_balance.is_none());

    // Bob attaches far more than registration costs
    let bob_balance_before = near.balance(bob.account_id()).await?.total;
    let ft_balance_before = near.balance(&ft_contract_id).await?.total;

    bob.transaction(&ft_contract_id)
        .add_action(
            FunctionCall::new("storage_deposit")
                .gas(Gas::from_ggas(1500))
                .deposit(NearToken::from_near(5)),
        )
        .await?
        .result()?;

    // The token contract keeps only `STORAGE_DEPOSIT` and refunds the rest
    let storage_balance: Option<StorageBalance> = near
        .view(&ft_contract_id, "storage_balance_of")
        .args(json!({ "account_id": bob.account_id() }))
        .await?;

    let storage_balance = storage_balance.expect("Bob should be registered");

    assert_eq!(storage_balance.total, STORAGE_DEPOSIT);
    assert_eq!(storage_balance.available, NearToken::ZERO);

    // Bob pays `STORAGE_DEPOSIT` and the transaction fee
    let bob_spent = bob_balance_before.saturating_sub(near.balance(bob.account_id()).await?.total);

    assert!(bob_spent > STORAGE_DEPOSIT);
    assert!(bob_spent < STORAGE_DEPOSIT.saturating_add(NearToken::from_millinear(1)));

    // The token contract gets exactly `STORAGE_DEPOSIT`
    let ft_received = near.balance(&ft_contract_id).await?.total.saturating_sub(ft_balance_before);

    assert_eq!(ft_received, STORAGE_DEPOSIT);

    Ok(())
}
