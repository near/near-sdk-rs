use std::collections::BTreeMap;

use near_kit::protocol::Action;
use near_kit::rpc::STORAGE_AMOUNT_PER_BYTE;
use near_kit::signer::InMemorySigner;
use near_kit::transaction::ExecutedOptimistic;
use near_kit::{AccountId, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use tokio::sync::Mutex;

/// Global contract code costs 10 times more storage per byte than regular contract code.
pub const GLOBAL_STORAGE_COST_PER_BYTE: NearToken = STORAGE_AMOUNT_PER_BYTE.saturating_mul(10);

/// The wasm of each contract `build_contract` has built in this process, by path.
static BUILT_CONTRACTS: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::const_new(BTreeMap::new());

/// Builds the contract in `path` (relative to this crate) with `cargo near` and returns the wasm.
///
/// `cargo test` runs all the tests of this binary in one process, so each contract is built once
/// and the tests share its wasm.
pub async fn build_contract(path: &str) -> anyhow::Result<Vec<u8>> {
    // Keep the lock until the build is done, so other tests wait for it instead of building the
    // same contract again
    let mut built_contracts = BUILT_CONTRACTS.lock().await;

    if let Some(wasm) = built_contracts.get(path) {
        return Ok(wasm.clone());
    }

    let manifest = cargo_near_build::camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(path)
        .join("Cargo.toml");

    let wasm = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
            no_locked: true,
            manifest_path: Some(manifest),
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("cargo near build: {e:?}"))?;

        Ok(std::fs::read(&artifact)?)
    })
    .await??;

    built_contracts.insert(path.to_string(), wasm.clone());

    Ok(wasm)
}

/// Starts a node with Alice in genesis and deploys the factory to `factory.alice`.
/// Alice pays for the setup and calls the factory.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
pub async fn setup() -> anyhow::Result<(Sandbox, SigningNear<ExecutedOptimistic>, AccountId)> {
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
