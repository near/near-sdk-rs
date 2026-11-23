use std::collections::BTreeMap;

use near_kit::json_types::U128;
use near_kit::protocol::{Action, ActionErrorKind, FunctionCallError};
use near_kit::rpc::{FinalExecutionOutcome, Finality, STORAGE_AMOUNT_PER_BYTE};
use near_kit::signer::InMemorySigner;
use near_kit::transaction::{ExecutedOptimistic, FunctionCall};
use near_kit::{AccountId, Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde_json::json;
use tokio::sync::Mutex;

/// One whole token: the token metadata sets 24 decimals.
pub const ONE_TOKEN: u128 = 10u128.pow(24);

/// The token supply Alice owns after setup.
pub const TOTAL_SUPPLY: u128 = 10_000 * ONE_TOKEN;

/// What the token contract charges to register one account.
pub const STORAGE_DEPOSIT: NearToken = STORAGE_AMOUNT_PER_BYTE.saturating_mul(125);

/// The deposit `ft_transfer`, `ft_transfer_call` and `storage_unregister` require.
pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

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
            // The tests don't use the ABI. Generating it needs crates that aren't in Cargo.lock,
            // so every build would query the crates.io index
            no_abi: true,
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

/// Starts a node with Alice and Bob in genesis and deploys the token contract to `ft.alice`.
/// Alice owns the whole token supply. Bob isn't registered with the token contract.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
pub async fn setup() -> anyhow::Result<(
    Sandbox,
    SigningNear<ExecutedOptimistic>,
    SigningNear<ExecutedOptimistic>,
    AccountId,
)> {
    let alice_account = GenesisAccount::default_with_name(account!("alice"));
    let bob_account = GenesisAccount::default_with_name(account!("bob"));

    let node = Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![alice_account.clone(), bob_account.clone()],
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

    let bob = client
        .with_signer(InMemorySigner::new(bob_account.account_id, bob_account.private_key.parse()?))
        .wait_until::<ExecutedOptimistic>();

    // Create the token account, fund it, deploy the token contract and mint the supply to Alice
    let wasm = build_contract("ft").await?;
    let ft_contract_id = alice.account_id().sub_account("ft")?;

    alice
        .transaction(&ft_contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .add_action(
            FunctionCall::new("new_default_meta")
                .args(json!({
                    "owner_id": alice.account_id(),
                    "total_supply": U128(TOTAL_SUPPLY),
                }))
                .gas(Gas::from_tgas(2)),
        )
        .await?
        .result()?;

    Ok((node, alice, bob, ft_contract_id))
}

/// Registers `account_id` with the token contract. `payer` pays the storage deposit.
pub async fn register(
    payer: &SigningNear<ExecutedOptimistic>,
    ft_contract_id: &AccountId,
    account_id: &AccountId,
) -> anyhow::Result<()> {
    payer
        .transaction(ft_contract_id)
        .add_action(
            FunctionCall::new("storage_deposit")
                .args(json!({ "account_id": account_id }))
                .gas(Gas::from_tgas(2))
                .deposit(STORAGE_DEPOSIT),
        )
        .await?
        .result()?;

    Ok(())
}

/// The token balance of `account_id`.
pub async fn ft_balance_of(
    near: &Near,
    ft_contract_id: &AccountId,
    account_id: &AccountId,
) -> anyhow::Result<u128> {
    let balance: U128 = near
        .view(ft_contract_id, "ft_balance_of")
        .args(json!({ "account_id": account_id }))
        .at(Finality::Optimistic)
        .await?;

    Ok(balance.0)
}

/// Every failed receipt in `tx_details`: the message for a contract panic,
/// the whole error otherwise.
pub fn failures(tx_details: &FinalExecutionOutcome) -> Vec<String> {
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
