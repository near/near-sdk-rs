use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;

use near_sdk::ONE_YOCTO;

#[tokio::test]
async fn simulate_simple_transfer() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, alice, _, _) = init(&worker).await?;

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    let res = nft_contract
        .call("nft_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("simple transfer".to_string()),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // A single NFT transfer event should have been logged:
    assert_eq!(res.logs().len(), 1);

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), alice.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_fast_return_to_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "return-it-now",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_slow_return_to_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "return-it-later",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_fast_keep_with_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "keep-it-now",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());
    assert_eq!(res.logs().len(), 2);

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), token_receiver_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_slow_keep_with_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "keep-it-later",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), token_receiver_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_receiver_panics() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "incorrect message",
        ))
        .gas(35_000_000_000_000 + 1)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Prints final logs
    assert_eq!(res.logs().len(), 3);

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_receiver_panics_and_nft_resolve_transfer_produces_no_log_if_not_enough_gas(
) -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "incorrect message",
        ))
        .gas(30_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_failure());

    // Prints no logs
    assert_eq!(res.logs().len(), 0);

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_simple_transfer_no_logs_on_failure() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, _, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_transfer")
        // transfer to the current owner should fail and not print log
        .args_json((nft_contract.id(), TOKEN_ID, Option::<u64>::None, Some("simple transfer")))
        .gas(200_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_failure());

    // Prints no logs
    assert_eq!(res.logs().len(), 0);

    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}
