use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::U128;
use near_sdk::NearToken;

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

#[tokio::test]
async fn simulate_payout() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, alice, _, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_payout")
        .args_json((TOKEN_ID, U128::from(1), Option::<u32>::None))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    // A single NFT transfer event should have been logged:
    assert_eq!(res.logs().len(), 0);

    Ok(())
}

#[tokio::test]
async fn nft_transfer_payout() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, alice, _, _) = init(&worker).await?;

    let token =
        nft_contract.call("nft_token").args_json((TOKEN_ID,)).view().await?.json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    let res = nft_contract
        .call("nft_transfer_payout")
        .args_json((
            alice.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("simple transfer".to_string()),
            U128::from(1),
            Option::<u32>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_success());

    // A single NFT transfer event should have been logged:
    assert_eq!(res.logs().len(), 1);

    let token =
        nft_contract.call("nft_token").args_json((TOKEN_ID,)).view().await?.json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), alice.id().to_string());

    Ok(())
}
