use std::str::FromStr;

use near_contract_standards::multi_token::metadata::MTTokenMetadata;
use near_contract_standards::multi_token::TokenId;

use near_workspaces::cargo_near_build;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use rstest::fixture;

pub const TOKEN_ID_SWORD: &str = "sword-001";
pub const TOKEN_ID_POTION: &str = "potion-001";
pub const TOKEN_ID_GOLD: &str = "gold-001";

pub const INITIAL_MINT_AMOUNT: u128 = 1000;
pub const MINT_STORAGE_DEPOSIT: NearToken = NearToken::from_millinear(10);

pub fn sample_token_metadata(title: &str, description: &str) -> MTTokenMetadata {
    MTTokenMetadata {
        title: Some(title.to_string()),
        description: Some(description.to_string()),
        media: None,
        media_hash: None,
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    }
}

pub async fn helper_mint(
    mt_contract: &Contract,
    caller: &Account,
    token_id: TokenId,
    owner_id: &near_workspaces::AccountId,
    amount: u128,
    metadata: Option<MTTokenMetadata>,
) -> anyhow::Result<()> {
    let res = caller
        .call(mt_contract.id(), "mt_mint")
        .args_json((token_id, owner_id, near_sdk::json_types::U128(amount), metadata))
        .max_gas()
        .deposit(MINT_STORAGE_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success(), "Mint failed: {:?}", res.failures());
    Ok(())
}

fn build_contract(path: &str, contract_name: &str) -> Vec<u8> {
    let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
        manifest_path: Some(
            cargo_near_build::camino::Utf8PathBuf::from_str(path).expect("camino PathBuf from str"),
        ),
        ..Default::default()
    })
    .unwrap_or_else(|e| panic!("building `{}` contract for tests: {:?}", contract_name, e));

    let contract_wasm = std::fs::read(&artifact)
        .map_err(|err| format!("accessing {} to read wasm contents: {}", artifact, err))
        .expect("std::fs::read");
    contract_wasm
}

#[fixture]
#[once]
fn multi_token_contract_wasm() -> Vec<u8> {
    build_contract("./mt/Cargo.toml", "multi-token")
}

#[fixture]
#[once]
fn token_receiver_contract_wasm() -> Vec<u8> {
    build_contract("./test-token-receiver/Cargo.toml", "test-mt-receiver")
}

/// Deploy and initialize contracts and return:
/// * mt_contract: the Multi Token contract
/// * alice: a user account
/// * bob: another user account
/// * token_receiver_contract: a contract implementing `mt_on_transfer`
#[fixture]
pub async fn initialized_contracts(
    multi_token_contract_wasm: &Vec<u8>,
    token_receiver_contract_wasm: &Vec<u8>,
) -> anyhow::Result<(Contract, Account, Account, Contract)> {
    let worker = near_workspaces::sandbox().await?;
    let mt_contract = worker.dev_deploy(multi_token_contract_wasm).await?;

    // Initialize MT contract
    let res = mt_contract
        .call("new_default_meta")
        .args_json((mt_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success(), "Init failed: {:?}", res.failures());

    // Mint initial tokens for testing
    let metadata = sample_token_metadata("Silver Sword", "A legendary sword");
    let res = mt_contract
        .call("mt_mint")
        .args_json((
            TOKEN_ID_SWORD,
            mt_contract.id(),
            near_sdk::json_types::U128(INITIAL_MINT_AMOUNT),
            Some(metadata),
        ))
        .max_gas()
        .deposit(MINT_STORAGE_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success(), "Mint sword failed: {:?}", res.failures());

    // Mint potions (fungible-style)
    let metadata = sample_token_metadata("Health Potion", "Restores 50 HP");
    let res = mt_contract
        .call("mt_mint")
        .args_json((
            TOKEN_ID_POTION,
            mt_contract.id(),
            near_sdk::json_types::U128(INITIAL_MINT_AMOUNT * 10),
            Some(metadata),
        ))
        .max_gas()
        .deposit(MINT_STORAGE_DEPOSIT)
        .transact()
        .await?;
    assert!(res.is_success(), "Mint potion failed: {:?}", res.failures());

    // Create alice
    let res = mt_contract
        .as_account()
        .create_subaccount("alice")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?;
    assert!(res.is_success());
    let alice = res.result;

    // Create bob
    let res = mt_contract
        .as_account()
        .create_subaccount("bob")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?;
    assert!(res.is_success());
    let bob = res.result;

    // Deploy token receiver
    let token_receiver_contract = worker.dev_deploy(token_receiver_contract_wasm).await?;
    let res = token_receiver_contract
        .call("new")
        .args_json((mt_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success(), "Token receiver init failed: {:?}", res.failures());

    Ok((mt_contract, alice, bob, token_receiver_contract))
}
