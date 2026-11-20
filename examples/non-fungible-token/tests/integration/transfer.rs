use near_kit::Gas;
use near_kit::transaction::FunctionCall;
use serde_json::json;

use crate::shared::{NftTransfer, ONE_YOCTO, TOKEN_ID, failures, nft_token, nft_transfers, setup};

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;
    let near = alice.near();

    assert_eq!(nft_token(near, &nft_contract_id).await?.owner_id, *alice.account_id());

    let tx_details = alice
        .transaction(&nft_contract_id)
        .add_action(
            FunctionCall::new("nft_transfer")
                .args(json!({
                    "receiver_id": bob.account_id(),
                    "token_id": TOKEN_ID,
                    "memo": "simple transfer",
                }))
                .gas(Gas::from_tgas(3))
                .deposit(ONE_YOCTO),
        )
        .await?
        .checked()?;

    // The token contract logs a single transfer event
    assert_eq!(
        nft_transfers(&tx_details, &nft_contract_id)?,
        [NftTransfer::new(alice.account_id(), bob.account_id(), Some("simple transfer"))]
    );

    assert_eq!(nft_token(near, &nft_contract_id).await?.owner_id, *bob.account_id());

    Ok(())
}

#[tokio::test]
async fn test_simple_transfer_no_logs_on_failure() -> anyhow::Result<()> {
    let (_node, alice, _, nft_contract_id) = setup().await?;

    // Alice can't transfer the token to herself, its current owner
    let tx_details = alice
        .transaction(&nft_contract_id)
        .add_action(
            FunctionCall::new("nft_transfer")
                .args(json!({
                    "receiver_id": alice.account_id(),
                    "token_id": TOKEN_ID,
                    "memo": "simple transfer",
                }))
                .gas(Gas::from_tgas(3))
                .deposit(ONE_YOCTO),
        )
        .await?;

    assert_eq!(failures(&tx_details), ["Current and next owner must differ"]);

    // The failed call logs nothing at all
    assert_eq!(tx_details.logs().count(), 0);

    assert_eq!(nft_token(alice.near(), &nft_contract_id).await?.owner_id, *alice.account_id());

    Ok(())
}
