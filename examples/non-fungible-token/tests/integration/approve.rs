use std::collections::HashMap;

use near_kit::Gas;
use near_kit::transaction::FunctionCall;
use serde_json::json;

use crate::shared::{
    ONE_YOCTO, TOKEN_ID, approval_deposit, approve, deploy_receiver, is_approved, nft_token, setup,
};

#[tokio::test]
async fn test_simple_approve() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;
    let near = alice.near();

    let approval_receiver_id =
        deploy_receiver(&alice, &nft_contract_id, "test-approval-receiver", "approval-receiver")
            .await?;

    // Alice approves Bob, who gets approval id 1
    alice.transaction(&nft_contract_id).add_action(approve(bob.account_id())).await?.result()?;

    // `nft_is_approved` checks the approval id only when it's given one
    for (approval_id, approved) in [(None, true), (Some(1), true), (Some(2), false)] {
        assert_eq!(
            is_approved(near, &nft_contract_id, bob.account_id(), approval_id).await?,
            approved,
            "nft_is_approved({approval_id:?})"
        );
    }

    // `nft_token` lists the approvals too
    let token = nft_token(near, &nft_contract_id).await?;

    assert_eq!(
        token.approved_account_ids,
        Some(HashMap::from([(bob.account_id().to_string(), 1)]))
    );

    // Approving Bob again gives him a new approval id. It stores nothing new,
    // so 1 yoctoNEAR is enough.
    alice
        .transaction(&nft_contract_id)
        .add_action(approve(bob.account_id()).deposit(ONE_YOCTO))
        .await?
        .result()?;

    assert!(is_approved(near, &nft_contract_id, bob.account_id(), Some(2)).await?);

    // Another account gets the next approval id
    alice
        .transaction(&nft_contract_id)
        .add_action(approve(&approval_receiver_id))
        .await?
        .result()?;

    assert!(is_approved(near, &nft_contract_id, &approval_receiver_id, Some(3)).await?);

    Ok(())
}

#[tokio::test]
async fn test_approval_with_call() -> anyhow::Result<()> {
    let (_node, alice, _, nft_contract_id) = setup().await?;

    let approval_receiver_id =
        deploy_receiver(&alice, &nft_contract_id, "test-approval-receiver", "approval-receiver")
            .await?;

    // With a `msg`, `nft_approve` calls `nft_on_approve` on the approved account and returns
    // its result. For "return-now", the approval receiver returns "cool" right away.
    let returned: String = alice
        .transaction(&nft_contract_id)
        .add_action(
            FunctionCall::new("nft_approve")
                .args(json!({
                    "token_id": TOKEN_ID,
                    "account_id": approval_receiver_id,
                    "msg": "return-now",
                }))
                .gas(Gas::from_tgas(3))
                .deposit(approval_deposit(&approval_receiver_id)),
        )
        .await?
        .json()?;

    assert_eq!(returned, "cool");

    // Approving again sets a new approval id, which the approval receiver ignores.
    // For any other `msg`, it returns the `msg` after one more cross-contract call.
    let returned: String = alice
        .transaction(&nft_contract_id)
        .add_action(
            FunctionCall::new("nft_approve")
                .args(json!({
                    "token_id": TOKEN_ID,
                    "account_id": approval_receiver_id,
                    "msg": "hahaha",
                }))
                .gas(Gas::from_tgas(5))
                .deposit(ONE_YOCTO),
        )
        .await?
        .json()?;

    assert_eq!(returned, "hahaha");

    Ok(())
}

#[tokio::test]
async fn test_approved_account_transfers_token() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;

    // Alice approves Bob, who gets approval id 1
    alice.transaction(&nft_contract_id).add_action(approve(bob.account_id())).await?.result()?;

    // Bob sends Alice's token to himself
    bob.transaction(&nft_contract_id)
        .add_action(
            FunctionCall::new("nft_transfer")
                .args(json!({
                    "receiver_id": bob.account_id(),
                    "token_id": TOKEN_ID,
                    "approval_id": 1,
                    "memo": "gotcha! bahahaha",
                }))
                .gas(Gas::from_tgas(3))
                .deposit(ONE_YOCTO),
        )
        .await?
        .result()?;

    assert_eq!(nft_token(bob.near(), &nft_contract_id).await?.owner_id, *bob.account_id());

    Ok(())
}
