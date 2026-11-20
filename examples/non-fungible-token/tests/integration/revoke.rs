use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas};
use serde_json::json;

use crate::shared::{ONE_YOCTO, TOKEN_ID, approve, deploy_receiver, is_approved, setup};

/// `nft_revoke` that takes back the approval of `account_id` for `TOKEN_ID`.
fn revoke(account_id: &AccountId) -> FunctionCall {
    FunctionCall::new("nft_revoke")
        .args(json!({
            "token_id": TOKEN_ID,
            "account_id": account_id,
        }))
        .gas(Gas::from_tgas(2))
        .deposit(ONE_YOCTO)
}

#[tokio::test]
async fn test_revoke() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;
    let near = alice.near();

    let approval_receiver_id =
        deploy_receiver(&alice, &nft_contract_id, "test-approval-receiver", "approval-receiver")
            .await?;

    // Alice approves Bob and the approval receiver
    alice
        .transaction(&nft_contract_id)
        .add_action(approve(bob.account_id()))
        .add_action(approve(&approval_receiver_id))
        .await?
        .result()?;

    // Alice revokes Bob, but the approval receiver stays approved
    alice.transaction(&nft_contract_id).add_action(revoke(bob.account_id())).await?.result()?;

    assert!(!is_approved(near, &nft_contract_id, bob.account_id(), None).await?);
    assert!(is_approved(near, &nft_contract_id, &approval_receiver_id, None).await?);

    // Alice revokes the approval receiver too
    alice
        .transaction(&nft_contract_id)
        .add_action(revoke(&approval_receiver_id))
        .await?
        .result()?;

    assert!(!is_approved(near, &nft_contract_id, bob.account_id(), None).await?);
    assert!(!is_approved(near, &nft_contract_id, &approval_receiver_id, None).await?);

    Ok(())
}

#[tokio::test]
async fn test_revoke_all() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;
    let near = alice.near();

    let approval_receiver_id =
        deploy_receiver(&alice, &nft_contract_id, "test-approval-receiver", "approval-receiver")
            .await?;

    // Alice approves Bob and the approval receiver
    alice
        .transaction(&nft_contract_id)
        .add_action(approve(bob.account_id()))
        .add_action(approve(&approval_receiver_id))
        .await?
        .result()?;

    // Alice revokes all the approvals at once
    alice
        .transaction(&nft_contract_id)
        .add_action(
            FunctionCall::new("nft_revoke_all")
                .args(json!({ "token_id": TOKEN_ID }))
                .gas(Gas::from_tgas(2))
                .deposit(ONE_YOCTO),
        )
        .await?
        .result()?;

    assert!(!is_approved(near, &nft_contract_id, bob.account_id(), None).await?);
    assert!(!is_approved(near, &nft_contract_id, &approval_receiver_id, None).await?);

    Ok(())
}
