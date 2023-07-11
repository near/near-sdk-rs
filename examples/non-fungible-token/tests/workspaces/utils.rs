use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_contract_standards::non_fungible_token::TokenId;

use near_units::parse_near;
use workspaces::{Account, Contract, DevNetwork, Worker};

pub const TOKEN_ID: &str = "0";

pub async fn helper_mint(
    nft_contract: &Contract,
    token_id: TokenId,
    title: String,
    desc: String,
) -> anyhow::Result<()> {
    let token_metadata = TokenMetadata {
        title: Some(title),
        description: Some(desc),
        media: None,
        media_hash: None,
        copies: Some(1u64),
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    };
    let res = nft_contract
        .call("nft_mint")
        .args_json((token_id, nft_contract.id(), token_metadata))
        .max_gas()
        .deposit(parse_near!("7 mN"))
        .transact()
        .await?;
    assert!(res.is_success());

    Ok(())
}

/// Deploy and initialize contracts and return:
/// * nft_contract: the NFT contract, callable with `call!` and `view!`
/// * alice: a user account, does not yet own any tokens
/// * token_receiver_contract: a contract implementing `nft_on_transfer` for use with `transfer_and_call`
/// * approval_receiver_contract: a contract implementing `nft_on_approve` for use with `nft_approve`
pub async fn init(
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<(Contract, Account, Contract, Contract)> {
    let nft_contract =
        worker.dev_deploy(include_bytes!("../../res/non_fungible_token.wasm")).await?;

    let res = nft_contract
        .call("new_default_meta")
        .args_json((nft_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let token_metadata = TokenMetadata {
        title: Some("Olympus Mons".into()),
        description: Some("The tallest mountain in the charted solar system".into()),
        media: None,
        media_hash: None,
        copies: Some(1u64),
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    };
    let res = nft_contract
        .call("nft_mint")
        .args_json((TOKEN_ID, nft_contract.id(), token_metadata))
        .max_gas()
        .deposit(parse_near!("7 mN"))
        .transact()
        .await?;
    assert!(res.is_success());

    let res = nft_contract
        .as_account()
        .create_subaccount("alice")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?;
    assert!(res.is_success());
    let alice = res.result;

    let token_receiver_contract =
        worker.dev_deploy(include_bytes!("../../res/token_receiver.wasm")).await?;
    let res = token_receiver_contract
        .call("new")
        .args_json((nft_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let approval_receiver_contract =
        worker.dev_deploy(include_bytes!("../../res/approval_receiver.wasm")).await?;
    let res = approval_receiver_contract
        .call("new")
        .args_json((nft_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    return Ok((nft_contract, alice, token_receiver_contract, approval_receiver_contract));
}
