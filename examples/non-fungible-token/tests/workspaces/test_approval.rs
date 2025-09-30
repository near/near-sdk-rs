use crate::utils::initialized_contracts;
use near_contract_standards::non_fungible_token::Token;

use near_sdk::AccountId;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use rstest::rstest;
use std::collections::HashMap;
use std::convert::TryFrom;

pub const TOKEN_ID: &str = "0";

const ONE_NEAR: NearToken = NearToken::from_near(1);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

#[rstest]
#[tokio::test]
async fn simulate_simple_approve(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Contract, Contract)>,
) -> anyhow::Result<()> {
    let (nft_contract, alice, token_receiver_contract, _) = initialized_contracts.await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(610000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // check nft_is_approved, don't provide approval_id
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approved);

    // check nft_is_approved, with approval_id=1
    let alice_approval_id_is_1 = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(1u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approval_id_is_1);

    // check nft_is_approved, with approval_id=2
    let alice_approval_id_is_2 = nft_contract
        .call("nft_is_approved")
        .args_json(&(TOKEN_ID, alice.id(), Some(2u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approval_id_is_2);

    // alternatively, one could check the data returned by nft_token
    let token =
        nft_contract.call("nft_token").args_json((TOKEN_ID,)).view().await?.json::<Token>()?;
    let mut expected_approvals: HashMap<AccountId, u64> = HashMap::new();
    expected_approvals.insert(AccountId::try_from(alice.id().to_string())?, 1);
    assert_eq!(token.approved_account_ids.unwrap(), expected_approvals);

    // root approves alice again, which changes the approval_id and doesn't require as much deposit
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(ONE_NEAR)
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    let alice_approval_id_is_2 = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(2u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approval_id_is_2);

    // approving another account gives different approval_id
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<String>::None))
        .max_gas()
        // note that token_receiver's account name is shorter, and so takes less bytes to store and
        // therefore requires a smaller deposit!
        .deposit(NearToken::from_yoctonear(550000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    let token_receiver_approval_id_is_3 = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(token_receiver_approval_id_is_3);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn simulate_approval_with_call(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Contract, Contract)>,
) -> anyhow::Result<()> {
    let (nft_contract, _, _, approval_receiver_contract) = initialized_contracts.await?;

    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, approval_receiver_contract.id(), Some("return-now".to_string())))
        .max_gas()
        .deposit(NearToken::from_yoctonear(550000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());
    assert_eq!(res.json::<String>()?, "cool".to_string());

    // Approve again; will set different approval_id (ignored by approval_receiver).
    // The approval_receiver implementation will return given `msg` after subsequent promise call,
    // if given something other than "return-now".
    let msg = "hahaha".to_string();
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, approval_receiver_contract.id(), Some(msg.clone())))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());
    assert_eq!(res.json::<String>()?, msg);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn simulate_approved_account_transfers_token(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Contract, Contract)>,
) -> anyhow::Result<()> {
    let (nft_contract, alice, _, _) = initialized_contracts.await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(610000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // alice sends to self
    let res = alice
        .call(nft_contract.id(), "nft_transfer")
        .args_json((alice.id(), TOKEN_ID, Some(1u64), Some("gotcha! bahahaha".to_string())))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // token now owned by alice
    let token =
        nft_contract.call("nft_token").args_json((TOKEN_ID,)).view().await?.json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), alice.id().to_string());

    Ok(())
}

#[rstest]
#[tokio::test]
async fn simulate_revoke(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Contract, Contract)>,
) -> anyhow::Result<()> {
    let (nft_contract, alice, token_receiver_contract, _) = initialized_contracts.await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(610000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // root approves token_receiver
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(550000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // root revokes alice
    let res = nft_contract
        .call("nft_revoke")
        .args_json((TOKEN_ID, alice.id()))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // alice is revoked...
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // but token_receiver is still approved
    let token_receiver_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(token_receiver_approved);

    // root revokes token_receiver
    let res = nft_contract
        .call("nft_revoke")
        .args_json((TOKEN_ID, token_receiver_contract.id()))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // alice is still revoked...
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // ...and now so is token_receiver
    let token_receiver_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!token_receiver_approved,);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn simulate_revoke_all(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Contract, Contract)>,
) -> anyhow::Result<()> {
    let (nft_contract, alice, token_receiver_contract, _) = initialized_contracts.await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(610000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // root approves token_receiver
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(550000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // root revokes all
    let res = nft_contract
        .call("nft_revoke_all")
        .args_json((TOKEN_ID,))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "{}", res.into_result().unwrap_err().to_string());

    // alice is revoked...
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // and so is token_receiver
    let token_receiver_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!token_receiver_approved);

    Ok(())
}
