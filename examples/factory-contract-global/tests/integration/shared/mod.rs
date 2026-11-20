use near_kit::protocol::Action;
use near_kit::rpc::STORAGE_AMOUNT_PER_BYTE;
use near_kit::signer::InMemorySigner;
use near_kit::{AccountId, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};

/// Global contract code costs 10 times more storage per byte than regular contract code.
pub const GLOBAL_STORAGE_COST_PER_BYTE: NearToken = STORAGE_AMOUNT_PER_BYTE.saturating_mul(10);

/// Builds the contract in `path` (relative to this crate) with `cargo near` and returns the wasm.
pub async fn build_contract(path: &str) -> anyhow::Result<Vec<u8>> {
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

/// Starts a node with Alice in genesis and deploys the factory to `factory.alice`.
/// Alice pays for the setup and calls the factory.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
pub async fn setup() -> anyhow::Result<(Sandbox, SigningNear, AccountId)> {
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

    // Create the factory account, fund it and deploy the factory contract
    let wasm = build_contract(".").await?;
    let factory_id = alice.account_id().sub_account("factory")?;

    alice
        .transaction(&factory_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    Ok((node, alice, factory_id))
}
