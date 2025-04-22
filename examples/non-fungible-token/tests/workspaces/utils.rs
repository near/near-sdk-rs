use std::str::FromStr;

use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_contract_standards::non_fungible_token::TokenId;

use near_workspaces::cargo_near_build;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use rstest::fixture;
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
        .deposit(NearToken::from_millinear(7))
        .transact()
        .await?;
    assert!(res.is_success());

    Ok(())
}

fn build_contract(path: &str, contract_name: &str) -> Vec<u8> {
    let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
        manifest_path: Some(
            cargo_near_build::camino::Utf8PathBuf::from_str(path).expect("camino PathBuf from str"),
        ),
        ..Default::default()
    })
    .expect(&format!("building `{}` contract for tests", contract_name));

    let contract_wasm = std::fs::read(&artifact)
        .map_err(|err| format!("accessing {} to read wasm contents: {}", artifact, err))
        .expect("std::fs::read");
    contract_wasm
}

#[fixture]
#[once]
fn non_fungible_contract_wasm() -> Vec<u8> {
    build_contract("./nft/Cargo.toml", "non-fungible-token")
}

#[fixture]
#[once]
fn token_receiver_contract_wasm() -> Vec<u8> {
    build_contract("./test-token-receiver/Cargo.toml", "token-receiver")
}

#[fixture]
#[once]
fn approval_receiver_contract_wasm() -> Vec<u8> {
    build_contract("./test-approval-receiver/Cargo.toml", "approval-receiver")
}

/// Deploy and initialize contracts and return:
/// * nft_contract: the NFT contract, callable with `call!` and `view!`
/// * alice: a user account, does not yet own any tokens
/// * token_receiver_contract: a contract implementing `nft_on_transfer` for use with `transfer_and_call`
/// * approval_receiver_contract: a contract implementing `nft_on_approve` for use with `nft_approve`
#[fixture]
pub async fn initialized_contracts(
    non_fungible_contract_wasm: &Vec<u8>,
    token_receiver_contract_wasm: &Vec<u8>,
    approval_receiver_contract_wasm: &Vec<u8>,
) -> anyhow::Result<(Contract, Account, Contract, Contract)> {
    let worker = near_workspaces::sandbox().await?;
    let nft_contract = worker.dev_deploy(non_fungible_contract_wasm).await?;

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
        .deposit(NearToken::from_millinear(7))
        .transact()
        .await?;
    assert!(res.is_success());

    let res = nft_contract
        .as_account()
        .create_subaccount("alice")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?;
    assert!(res.is_success());
    let alice = res.result;

    let token_receiver_contract = worker.dev_deploy(token_receiver_contract_wasm).await?;
    let res = token_receiver_contract
        .call("new")
        .args_json((nft_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let approval_receiver_contract = worker.dev_deploy(approval_receiver_contract_wasm).await?;
    let res = approval_receiver_contract
        .call("new")
        .args_json((nft_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    return Ok((nft_contract, alice, token_receiver_contract, approval_receiver_contract));
}
