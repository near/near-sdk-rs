use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;
use near_primitives::views::FinalExecutionStatus;
use near_sdk::ONE_YOCTO;

#[tokio::test]
async fn simulate_simple_transfer() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, alice, _, _) = init(&worker).await?;

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    let res = nft_contract
        .call(&worker, "nft_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("simple transfer".to_string()),
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

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
async fn simulate_transfer_call_fast_return_to_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "return-it-now",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_slow_return_to_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "return-it-later",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_fast_keep_with_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "keep-it-now",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), token_receiver_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_slow_keep_with_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "keep-it-later",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), token_receiver_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_receiver_panics() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "incorrect message",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}
