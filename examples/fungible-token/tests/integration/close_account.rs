use near_kit::Gas;
use near_kit::json_types::U128;
use near_kit::rpc::Finality;
use near_kit::transaction::FunctionCall;
use serde_json::json;

use crate::shared::{ONE_YOCTO, failures, register, setup};

#[tokio::test]
async fn test_close_account_empty_balance() -> anyhow::Result<()> {
    let (_node, _, bob, ft_contract_id) = setup().await?;

    register(&bob, &ft_contract_id, bob.account_id()).await?;

    let closed: bool = bob
        .transaction(&ft_contract_id)
        .add_action(
            FunctionCall::new("storage_unregister").gas(Gas::from_tgas(2)).deposit(ONE_YOCTO),
        )
        .await?
        .json()?;

    assert!(closed);

    Ok(())
}

#[tokio::test]
async fn test_close_account_non_empty_balance() -> anyhow::Result<()> {
    let (_node, alice, _, ft_contract_id) = setup().await?;

    // Alice holds the whole supply, so she can't close her account without `force`
    for args in [json!({}), json!({ "force": false })] {
        let tx_details = alice
            .transaction(&ft_contract_id)
            .add_action(
                FunctionCall::new("storage_unregister")
                    .args(&args)
                    .gas(Gas::from_tgas(2))
                    .deposit(ONE_YOCTO),
            )
            .await?;

        assert_eq!(
            failures(&tx_details),
            ["Can't unregister the account with the positive balance without force"],
            "storage_unregister({args})"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_close_account_force_non_empty_balance() -> anyhow::Result<()> {
    let (_node, alice, _, ft_contract_id) = setup().await?;

    // With `force`, Alice closes her account and her tokens are burnt
    let closed: bool = alice
        .transaction(&ft_contract_id)
        .add_action(
            FunctionCall::new("storage_unregister")
                .args(json!({ "force": true }))
                .gas(Gas::from_tgas(2))
                .deposit(ONE_YOCTO),
        )
        .await?
        .json()?;

    assert!(closed);

    let total_supply: U128 =
        alice.near().view(&ft_contract_id, "ft_total_supply").at(Finality::Optimistic).await?;

    assert_eq!(total_supply.0, 0);

    Ok(())
}
