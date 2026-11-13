use near_kit::protocol::Action;
use near_kit::signer::{InMemorySigner, SecretKey};
use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas, Near, NearToken};
use near_sandbox::Sandbox;
use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY};
use std::str::FromStr;
use test_case::test_case;

/// The most gas one transaction can attach.
const MAX_GAS: Gas = Gas::from_tgas(1_000);

async fn build_contract(path: &str) -> anyhow::Result<Vec<u8>> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let manifest = cargo_near_build::camino::Utf8PathBuf::from_str(&path)
            .map_err(|e| anyhow::anyhow!("camino: {e}"))?
            .join("Cargo.toml");
        let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
            no_locked: true,
            manifest_path: Some(manifest),
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("cargo near build: {e:?}"))?;
        Ok(std::fs::read(&artifact)?)
    })
    .await?
}

#[test_case("./high-level")]
#[test_case("./low-level")]
#[tokio::test]
async fn test_deploy_status_message(contract_path: &str) -> anyhow::Result<()> {
    let wasm = build_contract(contract_path).await?;
    let sandbox = Sandbox::start_sandbox().await?;

    // The sandbox's genesis account pays for the setup.
    let root_key = SecretKey::from_str(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY)?;

    let root = Near::custom(sandbox.rpc_addr.as_str(), "sandbox")
        .build()
        .with_signer(InMemorySigner::new(DEFAULT_GENESIS_ACCOUNT.to_owned(), root_key));

    // Create the factory account, fund it, give it a key and deploy the
    // contract, all in one transaction.
    let contract_id: AccountId = format!("factory.{}", root.account_id()).parse()?;
    let contract_key = SecretKey::generate_ed25519();

    root.transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(100)))
        .add_action(Action::add_full_access_key(contract_key.public_key()))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    let contract = root.with_signer(InMemorySigner::new(&contract_id, contract_key));

    let status_id: AccountId = format!("status.{contract_id}").parse()?;
    let status_amt = NearToken::from_near(20);

    let res = contract
        .transaction(&contract_id)
        .add_action(
            FunctionCall::new("deploy_status_message")
                .args((&status_id, status_amt))
                .gas(MAX_GAS)
                .deposit(NearToken::from_near(50)),
        )
        .await?;

    assert!(res.is_success());

    let message = "hello world from factory";

    let res = contract
        .transaction(&contract_id)
        .add_action(FunctionCall::new("complex_call").args((&status_id, message)).gas(MAX_GAS))
        .await?;

    assert!(res.is_success());

    let value = res.json::<String>()?;
    assert_eq!(message, value.trim_matches(|c| c == '"'));

    Ok(())
}
