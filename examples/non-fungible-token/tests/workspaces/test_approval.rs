use crate::utils::init;
use near_contract_standards::non_fungible_token::Token;
use near_primitives::views::FinalExecutionStatus;
use near_sdk::{AccountId, ONE_NEAR, ONE_YOCTO};
use std::collections::HashMap;
use std::convert::TryFrom;

pub const TOKEN_ID: &str = "0";

#[tokio::test]
async fn simulate_simple_approve() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, alice, token_receiver_contract, _) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(510000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // check nft_is_approved, don't provide approval_id
    let alice_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Option::<u64>::None))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approved);

    // check nft_is_approved, with approval_id=1
    let alice_approval_id_is_1 = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(1u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approval_id_is_1);

    // check nft_is_approved, with approval_id=2
    let alice_approval_id_is_2 = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json(&(TOKEN_ID, alice.id(), Some(2u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approval_id_is_2);

    // alternatively, one could check the data returned by nft_token
    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    let mut expected_approvals: HashMap<AccountId, u64> = HashMap::new();
    expected_approvals.insert(AccountId::try_from(alice.id().to_string())?, 1);
    assert_eq!(token.approved_account_ids.unwrap(), expected_approvals);

    // root approves alice again, which changes the approval_id and doesn't require as much deposit
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(ONE_NEAR)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let alice_approval_id_is_2 = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(2u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approval_id_is_2);

    // approving another account gives different approval_id
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        // note that token_receiver's account name is shorter, and so takes less bytes to store and
        // therefore requires a smaller deposit!
        .deposit(450000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token_receiver_approval_id_is_3 = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Some(3u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(token_receiver_approval_id_is_3);

    Ok(())
}

#[tokio::test]
async fn simulate_approval_with_call() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, _, approval_receiver_contract) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, approval_receiver_contract.id(), Some("return-now".to_string())))?
        .gas(300_000_000_000_000)
        .deposit(450000000000000000000)
        .transact()
        .await?;
    assert_eq!(res.json::<String>()?, "cool".to_string());

    // Approve again; will set different approval_id (ignored by approval_receiver).
    // The approval_receiver implementation will return given `msg` after subsequent promise call,
    // if given something other than "return-now".
    let msg = "hahaha".to_string();
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, approval_receiver_contract.id(), Some(msg.clone())))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert_eq!(res.json::<String>()?, msg);

    Ok(())
}

#[tokio::test]
async fn simulate_approved_account_transfers_token() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, alice, _, _) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(510000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // alice sends to self
    let res = alice
        .call(&worker, nft_contract.id().clone(), "nft_transfer")
        .args_json((alice.id(), TOKEN_ID, Some(1u64), Some("gotcha! bahahaha".to_string())))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // token now owned by alice
    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), alice.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_revoke() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, alice, token_receiver_contract, _) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(510000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // root approves token_receiver
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(450000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // root revokes alice
    let res = nft_contract
        .call(&worker, "nft_revoke")
        .args_json((TOKEN_ID, alice.id()))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // alice is revoked...
    let alice_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // but token_receiver is still approved
    let token_receiver_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(token_receiver_approved);

    // root revokes token_receiver
    let res = nft_contract
        .call(&worker, "nft_revoke")
        .args_json((TOKEN_ID, token_receiver_contract.id()))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // alice is still revoked...
    let alice_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // ...and now so is token_receiver
    let token_receiver_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(!token_receiver_approved);

    Ok(())
}

#[tokio::test]
async fn simulate_revoke_all() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, alice, token_receiver_contract, _) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(510000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // root approves token_receiver
    let res = nft_contract
        .call(&worker, "nft_approve")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<String>::None))?
        .gas(300_000_000_000_000)
        .deposit(450000000000000000000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // root revokes all
    let res = nft_contract
        .call(&worker, "nft_revoke_all")
        .args_json((TOKEN_ID,))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // alice is revoked...
    let alice_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // and so is token_receiver
    let token_receiver_approved = nft_contract
        .call(&worker, "nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))?
        .view()
        .await?
        .json::<bool>()?;
    assert!(!token_receiver_approved);

    Ok(())
}
