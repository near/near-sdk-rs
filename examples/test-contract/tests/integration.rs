use near_abi::AbiRoot;
use near_kit::protocol::{Action, ActionErrorKind, FunctionCallError};
use near_kit::rpc::{FinalExecutionOutcome, Finality};
use near_kit::signer::{InMemorySigner, SecretKey};
use near_kit::transaction::{ExecutedOptimistic, Final, FunctionCall};
use near_kit::{Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde_json::json;

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

/// Starts a node with Alice in genesis and deploys this crate's contract, uninitialized,
/// to `test-contract.alice`. The contract account gets a key of its own, so the tests can
/// also sign transactions as the contract.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
async fn setup()
-> anyhow::Result<(Sandbox, SigningNear<ExecutedOptimistic>, SigningNear<ExecutedOptimistic>)> {
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

    // Create the contract account with its own key, fund it and deploy the contract
    let wasm = build_contract().await?;
    let contract_id = alice.account_id().sub_account("test-contract")?;
    let contract_key = SecretKey::generate_ed25519();

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::add_full_access_key(contract_key.public_key()))
        .add_action(Action::deploy_contract(wasm))
        // near-kit reads the signer's key from the final block, and the tests sign with this key
        .wait_until::<Final>()
        .await?
        .result()?;

    let contract = client
        .with_signer(InMemorySigner::new(contract_id, contract_key))
        .wait_until::<ExecutedOptimistic>();

    Ok((node, alice, contract))
}

/// Every failed receipt in `tx_details`: the message for a contract panic,
/// the whole error otherwise.
fn failures(tx_details: &FinalExecutionOutcome) -> Vec<String> {
    tx_details
        .receipt_failures()
        .map(|error| match &error.kind {
            ActionErrorKind::FunctionCallError(FunctionCallError::ExecutionError(message)) => {
                message.strip_prefix("Smart contract panicked: ").unwrap_or(message).to_string()
            }
            _ => error.to_string(),
        })
        .collect()
}

/// Checks that the contract is built with its ABI and serves it from the `__contract_abi` view.
#[tokio::test]
async fn test_embedded_abi() -> anyhow::Result<()> {
    let (_node, _, contract) = setup().await?;

    // `__contract_abi` returns the ABI as zstd-compressed JSON, so read the raw bytes
    let compressed_abi = contract
        .near()
        .view::<()>(contract.account_id(), "__contract_abi")
        .at(Finality::Optimistic)
        .fetch_raw()
        .await?
        .result;

    let abi: AbiRoot = serde_json::from_slice(&zstd::decode_all(compressed_abi.as_slice())?)?;

    assert_eq!(abi.schema_version, "0.4.0");
    assert_eq!(abi.metadata.name.as_deref(), Some("test-contract"));

    Ok(())
}

/// A private init method can't be called by another account.
#[tokio::test]
async fn test_private_init_cannot_be_called_by_external_account() -> anyhow::Result<()> {
    let (_node, alice, contract) = setup().await?;

    let tx_details = alice
        .transaction(contract.account_id())
        .add_action(
            FunctionCall::new("new_private").args(json!({ "value": 42 })).gas(Gas::from_tgas(1)),
        )
        .await?;

    assert_eq!(failures(&tx_details), ["Method new_private is private"]);

    Ok(())
}

/// A private init method can be called by the contract itself.
#[tokio::test]
async fn test_private_init_can_be_called_by_current_account() -> anyhow::Result<()> {
    let (_node, _, contract) = setup().await?;

    contract
        .transaction(contract.account_id())
        .add_action(
            FunctionCall::new("new_private").args(json!({ "value": 42 })).gas(Gas::from_tgas(1)),
        )
        .await?
        .result()?;

    let value: u32 =
        contract.near().view(contract.account_id(), "get_value").at(Finality::Optimistic).await?;

    assert_eq!(value, 42);

    Ok(())
}
