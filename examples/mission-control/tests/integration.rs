use near_abi::AbiRoot;
use near_kit::protocol::Action;
use near_kit::signer::InMemorySigner;
use near_kit::{Near, NearToken, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};

/// Builds this crate's contract with `cargo near` and returns the wasm.
/// `cargo near` embeds the contract's ABI in the wasm by default.
async fn build_contract() -> anyhow::Result<Vec<u8>> {
    tokio::task::spawn_blocking(|| -> anyhow::Result<Vec<u8>> {
        let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
            no_locked: true,
            manifest_path: Some(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml").into()),
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("cargo near build: {e:?}"))?;

        Ok(std::fs::read(&artifact)?)
    })
    .await?
}

/// Checks that the contract is built with its ABI and serves it from the `__contract_abi` view.
#[tokio::test]
async fn test_embedded_abi() -> anyhow::Result<()> {
    // Alice exists from genesis. She pays for the setup.
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
    let wasm = build_contract().await?;
    let contract_id = alice.account_id().sub_account("mission-control")?;

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    // `__contract_abi` returns the ABI as zstd-compressed JSON, so read the raw bytes
    let compressed_abi =
        client.view::<()>(&contract_id, "__contract_abi").fetch_raw().await?.result;

    let abi: AbiRoot = serde_json::from_slice(&zstd::decode_all(compressed_abi.as_slice())?)?;

    assert_eq!(abi.schema_version, "0.4.0");
    assert_eq!(abi.metadata.name.as_deref(), Some("mission-control"));

    Ok(())
}
