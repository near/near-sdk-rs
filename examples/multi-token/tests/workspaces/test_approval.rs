//! Integration tests for MultiToken approval functionality (NEP-245)
//! Tests: approve, revoke, revoke_all, is_approved, transfer with approval

use crate::utils::{
    initialized_contracts, INITIAL_MINT_AMOUNT, TOKEN_ID_POTION, TOKEN_ID_SWORD,
};
use near_sdk::json_types::U128;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use rstest::rstest;

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const APPROVAL_DEPOSIT: NearToken = NearToken::from_millinear(10);

// =============================================================================
// Approval Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_approve(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;
    let approval_amount = 500u128;

    // Owner approves alice
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(approval_amount)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success(), "Approve failed: {:?}", res.failures());

    // Check approval
    let is_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            alice.id(),
            vec![U128(approval_amount)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(is_approved);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_approve_insufficient_amount(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Approve for 100
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(100)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    // Check if approved for 200 - should be false
    let is_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            alice.id(),
            vec![U128(200)], // More than approved
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(!is_approved);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_batch_approve(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Approve multiple tokens
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            vec![U128(100), U128(500)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success(), "Batch approve failed: {:?}", res.failures());

    // Check both approved
    let is_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            alice.id(),
            vec![U128(100), U128(500)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(is_approved);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_revoke(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // First approve
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(100)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    // Then revoke
    let res = contract
        .call("mt_revoke")
        .args_json((vec![TOKEN_ID_SWORD], alice.id()))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Revoke failed: {:?}", res.failures());

    // Check no longer approved
    let is_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            alice.id(),
            vec![U128(1)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(!is_approved);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_revoke_all(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;

    // Approve both alice and bob
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(100)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(200)],
            bob.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    // Revoke all
    let res = contract
        .call("mt_revoke_all")
        .args_json((vec![TOKEN_ID_SWORD],))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Revoke all failed: {:?}", res.failures());

    // Neither should be approved now
    let alice_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            alice.id(),
            vec![U128(1)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(!alice_approved);

    let bob_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            bob.id(),
            vec![U128(1)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(!bob_approved);

    Ok(())
}

// =============================================================================
// Transfer with Approval Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_transfer_with_approval(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;
    let approval_amount = 200u128;

    // Owner approves alice to spend on their behalf
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(approval_amount)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    // Alice transfers from owner to bob using approval
    let res = alice
        .call(contract.id(), "mt_transfer")
        .args_json((
            bob.id(),
            TOKEN_ID_SWORD,
            U128(approval_amount),
            Some((contract.id().to_string(), 1u64)), // approval tuple
            Some("approved transfer"),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Approved transfer failed: {:?}", res.failures());

    // Verify balances
    let owner_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    let bob_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((bob.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;

    assert_eq!(owner_balance.0, INITIAL_MINT_AMOUNT - approval_amount);
    assert_eq!(bob_balance.0, approval_amount);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_with_insufficient_approval_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;

    // Approve alice for 50
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(50)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    // Alice tries to transfer 100 (more than approved)
    let res = alice
        .call(contract.id(), "mt_transfer")
        .args_json((
            bob.id(),
            TOKEN_ID_SWORD,
            U128(100),
            Some((contract.id().to_string(), 1u64)),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_without_approval_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;

    // Alice tries to transfer from owner without any approval
    let res = alice
        .call(contract.id(), "mt_transfer")
        .args_json((
            bob.id(),
            TOKEN_ID_SWORD,
            U128(100),
            Some((contract.id().to_string(), 1u64)),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_partial_approval_consumption(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;

    // Approve alice for 200
    let res = contract
        .call("mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(200)],
            alice.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success());

    // Alice transfers 80 (partial use of approval)
    let res = alice
        .call(contract.id(), "mt_transfer")
        .args_json((
            bob.id(),
            TOKEN_ID_SWORD,
            U128(80),
            Some((contract.id().to_string(), 1u64)),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Partial transfer failed: {:?}", res.failures());

    // Alice should still be approved for remaining 120
    let is_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            alice.id(),
            vec![U128(120)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(is_approved);

    // But not for 121
    let is_approved: bool = contract
        .call("mt_is_approved")
        .args_json((
            vec![TOKEN_ID_SWORD],
            alice.id(),
            vec![U128(121)],
            Option::<Vec<u64>>::None,
        ))
        .view()
        .await?
        .json()?;
    assert!(!is_approved);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_non_owner_cannot_approve(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;

    // Alice tries to approve bob for owner's tokens (should fail)
    let res = alice
        .call(contract.id(), "mt_approve")
        .args_json((
            vec![TOKEN_ID_SWORD],
            vec![U128(100)],
            bob.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(APPROVAL_DEPOSIT)
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}
