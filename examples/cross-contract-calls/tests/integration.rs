use near_kit::protocol::Action;
use near_kit::signer::InMemorySigner;
use near_kit::transaction::FunctionCall;
use near_kit::{Gas, Near, NearToken, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde_json::json;
use test_case::test_case;

/// Builds the contract in `path` (relative to this crate) with `cargo near` and returns the wasm.
async fn build_contract(path: &str) -> anyhow::Result<Vec<u8>> {
    let manifest = cargo_near_build::camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(path)
        .join("Cargo.toml");

    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
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

#[test_case("high-level", Gas::from_tgas(70) ; "high_level")]
#[test_case("low-level", Gas::from_tgas(280) ; "low_level")]
#[tokio::test]
async fn test_factorial(contract_path: &str, factorial_10_gas: Gas) -> anyhow::Result<()> {
    // Alice exists from genesis. She pays for the setup and calls the contract.
    let alice_account = GenesisAccount::default_with_name(account!("alice"));

    let node = Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![alice_account.clone()],
        ..Default::default()
    })
    .await?;

    let client = Near::custom(&node.rpc_addr, "sandbox").build();

    let alice = client.with_signer(InMemorySigner::new(
        alice_account.account_id,
        alice_account.private_key.parse()?,
    ));

    // Create the contract account, fund it and deploy the contract
    let wasm = build_contract(contract_path).await?;
    let contract_id = alice.account_id().sub_account("cross-contract")?;

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    // `factorial(1)` returns right away, without cross-contract calls.
    let one: u32 = alice
        .transaction(&contract_id)
        .add_action(FunctionCall::new("factorial").args(json!({ "n": 1 })).gas(Gas::from_tgas(1)))
        .await?
        .json()?;

    assert_eq!(one, 1);

    // `factorial(10)` calls itself down to `factorial(1)` and multiplies the results
    // back up in `factorial_mult` callbacks.
    let n: u32 = 10;

    let factorial: u32 = alice
        .transaction(&contract_id)
        .add_action(FunctionCall::new("factorial").args(json!({ "n": n })).gas(factorial_10_gas))
        .await?
        .json()?;

    assert_eq!(factorial, (1..=n).product::<u32>());

    Ok(())
}
