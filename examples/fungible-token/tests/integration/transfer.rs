use near_kit::Gas;
use near_kit::json_types::U128;
use near_kit::transaction::FunctionCall;
use serde_json::json;

use crate::shared::{ONE_TOKEN, ONE_YOCTO, TOTAL_SUPPLY, ft_balance_of, register, setup};

#[tokio::test]
async fn test_total_supply() -> anyhow::Result<()> {
    let (_node, alice, _, ft_contract_id) = setup().await?;

    let total_supply: U128 = alice.near().view(&ft_contract_id, "ft_total_supply").await?;

    assert_eq!(total_supply.0, TOTAL_SUPPLY);

    Ok(())
}

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let (_node, alice, bob, ft_contract_id) = setup().await?;
    let amount = 100 * ONE_TOKEN;

    // Bob can't hold tokens until he is registered with the token contract
    register(&bob, &ft_contract_id, bob.account_id()).await?;

    alice
        .transaction(&ft_contract_id)
        .add_action(
            FunctionCall::new("ft_transfer")
                .args(json!({
                    "receiver_id": bob.account_id(),
                    "amount": U128(amount),
                }))
                .gas(Gas::from_ggas(1200))
                .deposit(ONE_YOCTO),
        )
        .await?
        .result()?;

    let near = alice.near();

    assert_eq!(
        ft_balance_of(near, &ft_contract_id, alice.account_id()).await?,
        TOTAL_SUPPLY - amount
    );
    assert_eq!(ft_balance_of(near, &ft_contract_id, bob.account_id()).await?, amount);

    Ok(())
}
