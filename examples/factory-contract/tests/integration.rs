use near_kit::protocol::Action;
use near_kit::signer::InMemorySigner;
use near_kit::transaction::{ExecutedOptimistic, FunctionCall};
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
            // The tests don't use the ABI. Generating it needs crates that aren't in Cargo.lock,
            // so every build would query the crates.io index
            no_abi: true,
            manifest_path: Some(manifest),
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("cargo near build: {e:?}"))?;

        Ok(std::fs::read(&artifact)?)
    })
    .await?
}

#[test_case("high-level", Gas::from_tgas(10) ; "high_level")]
#[test_case("low-level", Gas::from_tgas(70) ; "low_level")]
#[tokio::test]
async fn test_deploy_status_message(
    contract_path: &str,
    complex_call_gas: Gas,
) -> anyhow::Result<()> {
    // Alice exists from genesis. She pays for the setup and calls the factory.
    let alice_account = GenesisAccount::default_with_name(account!("alice"));

    let node = Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![alice_account.clone()],
        ..Default::default()
    })
    .await?;

    let client = Near::custom(&node.rpc_addr, "sandbox").build();

    let alice = client
        .with_signer(InMemorySigner::new(
            alice_account.account_id,
            alice_account.private_key.parse()?,
        ))
        .wait_until::<ExecutedOptimistic>();

    // Create the factory account, fund it and deploy the factory contract
    let wasm = build_contract(contract_path).await?;
    let factory_id = alice.account_id().sub_account("factory")?;

    alice
        .transaction(&factory_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(100)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    // The factory creates `child-contract.factory.alice`, funds it with 20 NEAR, gives it
    // Alice's key and deploys the status-message contract there.
    let child_contract_id = factory_id.sub_account("child-contract")?;

    alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("deploy_status_message")
                .args(json!({
                    "account_id": child_contract_id,
                    "amount": NearToken::from_near(20),
                }))
                .gas(Gas::from_tgas(25)),
        )
        .await?
        .result()?;

    // `complex_call` sets Alice's status on the child contract and returns it.
    let message = "hello world from factory";

    let status: Option<String> = alice
        .transaction(&factory_id)
        .add_action(
            FunctionCall::new("complex_call")
                .args(json!({
                    "account_id": child_contract_id,
                    "message": message,
                }))
                .gas(complex_call_gas),
        )
        .await?
        .json()?;

    assert_eq!(status.as_deref(), Some(message));

    Ok(())
}
