use near_kit::transaction::{ExecutedOptimistic, FunctionCall};
use near_kit::{AccountId, Gas, SigningNear};
use near_sandbox::Sandbox;
use serde_json::json;
use test_case::test_case;

use crate::shared::{
    NftTransfer, ONE_YOCTO, TOKEN_ID, deploy_receiver, failures, nft_token, nft_transfers,
};

/// Runs the shared setup and deploys the token receiver contract to `token-receiver.alice`.
async fn setup() -> anyhow::Result<(Sandbox, SigningNear<ExecutedOptimistic>, AccountId, AccountId)>
{
    let (node, alice, _, nft_contract_id) = crate::shared::setup().await?;

    let token_receiver_id =
        deploy_receiver(&alice, &nft_contract_id, "test-token-receiver", "token-receiver").await?;

    Ok((node, alice, nft_contract_id, token_receiver_id))
}

/// `nft_transfer_call` that sends `TOKEN_ID` to `token_receiver_id` with `msg`.
///
/// The token receiver returns the token for `"return-it-now"` and `"return-it-later"`, keeps
/// it for `"keep-it-now"` and `"keep-it-later"` and panics on any other `msg`. For the
/// `-later` messages, it answers in a cross-contract call to itself.
fn transfer_call(token_receiver_id: &AccountId, msg: &str) -> FunctionCall {
    FunctionCall::new("nft_transfer_call")
        .args(json!({
            "receiver_id": token_receiver_id,
            "token_id": TOKEN_ID,
            "memo": "transfer & call",
            "msg": msg,
        }))
        .gas(Gas::from_tgas(15))
        .deposit(ONE_YOCTO)
}

#[test_case("return-it-now", false ; "return_it_now")]
#[test_case("return-it-later", false ; "return_it_later")]
#[test_case("keep-it-now", true ; "keep_it_now")]
#[test_case("keep-it-later", true ; "keep_it_later")]
#[tokio::test]
async fn test_transfer_call(msg: &str, kept: bool) -> anyhow::Result<()> {
    let (_node, alice, nft_contract_id, token_receiver_id) = setup().await?;

    let tx_details = alice
        .transaction(&nft_contract_id)
        .add_action(transfer_call(&token_receiver_id, msg))
        .await?
        .checked()?;

    // `nft_transfer_call` returns whether the receiver kept the token
    let transferred: bool = tx_details.json()?;

    assert_eq!(transferred, kept);

    let mut expected =
        vec![NftTransfer::new(alice.account_id(), &token_receiver_id, Some("transfer & call"))];

    if !kept {
        // The token contract gives the token back to Alice
        expected.push(NftTransfer::new(&token_receiver_id, alice.account_id(), None));
    }

    assert_eq!(nft_transfers(&tx_details, &nft_contract_id)?, expected);

    let owner_id = if kept { &token_receiver_id } else { alice.account_id() };

    assert_eq!(nft_token(alice.near(), &nft_contract_id).await?.owner_id, *owner_id);

    Ok(())
}

#[tokio::test]
async fn test_transfer_call_receiver_panics() -> anyhow::Result<()> {
    let (_node, alice, nft_contract_id, token_receiver_id) = setup().await?;

    // The token receiver panics on a `msg` it doesn't know, so the token goes back to Alice
    let tx_details = alice
        .transaction(&nft_contract_id)
        .add_action(transfer_call(&token_receiver_id, "incorrect message"))
        .await?;

    assert_eq!(failures(&tx_details), ["unsupported msg"]);

    let transferred: bool = tx_details.json()?;

    assert!(!transferred);

    assert_eq!(
        nft_transfers(&tx_details, &nft_contract_id)?,
        [
            NftTransfer::new(alice.account_id(), &token_receiver_id, Some("transfer & call")),
            NftTransfer::new(&token_receiver_id, alice.account_id(), None),
        ]
    );

    assert_eq!(nft_token(alice.near(), &nft_contract_id).await?.owner_id, *alice.account_id());

    Ok(())
}

#[tokio::test]
async fn test_transfer_call_not_enough_gas() -> anyhow::Result<()> {
    let (_node, alice, nft_contract_id, token_receiver_id) = setup().await?;

    // `nft_transfer_call` reserves 5 Tgas for `nft_resolve_transfer`, so with 5 Tgas in all
    // it fails before it calls the token receiver
    let tx_details = alice
        .transaction(&nft_contract_id)
        .add_action(transfer_call(&token_receiver_id, "keep-it-now").gas(Gas::from_tgas(5)))
        .await?;

    assert_eq!(failures(&tx_details), ["Exceeded the prepaid gas."]);

    // The failed call logs no events, and Alice keeps the token
    assert_eq!(nft_transfers(&tx_details, &nft_contract_id)?, []);
    assert_eq!(nft_token(alice.near(), &nft_contract_id).await?.owner_id, *alice.account_id());

    Ok(())
}
