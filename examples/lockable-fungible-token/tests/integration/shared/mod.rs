use near_kit::json_types::U128;
use near_kit::protocol::{Action, ActionErrorKind, FunctionCallError};
use near_kit::rpc::FinalExecutionOutcome;
use near_kit::signer::InMemorySigner;
use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde_json::json;
use tokio::sync::Mutex;

/// The token supply Alice owns after setup.
pub const TOTAL_SUPPLY: u128 = 10_000;

/// This crate's contract, once `build_contract` has built it in this process.
static BUILT_CONTRACT: Mutex<Option<Vec<u8>>> = Mutex::const_new(None);

/// Builds this crate's contract with `cargo near` and returns the wasm.
///
/// `cargo test` runs all the tests of this binary in one process, so the contract is built once
/// and the tests share its wasm.
pub async fn build_contract() -> anyhow::Result<Vec<u8>> {
    // Keep the lock until the build is done, so other tests wait for it instead of building the
    // contract again
    let mut built_contract = BUILT_CONTRACT.lock().await;

    if let Some(wasm) = built_contract.as_ref() {
        return Ok(wasm.clone());
    }

    let wasm = tokio::task::spawn_blocking(|| -> anyhow::Result<Vec<u8>> {
        let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
            no_locked: true,
            manifest_path: Some(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml").into()),
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("cargo near build: {e:?}"))?;

        Ok(std::fs::read(&artifact)?)
    })
    .await??;

    *built_contract = Some(wasm.clone());

    Ok(wasm)
}

/// Starts a node with Alice and Bob in genesis and deploys the token contract to
/// `lockable-ft.alice`. Alice owns the whole token supply, Bob has no tokens.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
pub async fn setup() -> anyhow::Result<(Sandbox, SigningNear, SigningNear, AccountId)> {
    let alice_account = GenesisAccount::default_with_name(account!("alice"));
    let bob_account = GenesisAccount::default_with_name(account!("bob"));

    let node = Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![alice_account.clone(), bob_account.clone()],
        ..Default::default()
    })
    .await?;

    let client = Near::custom(&node.rpc_addr, "sandbox").build();

    let alice = client.with_signer(InMemorySigner::new(
        alice_account.account_id,
        alice_account.private_key.parse()?,
    ));

    let bob = client
        .with_signer(InMemorySigner::new(bob_account.account_id, bob_account.private_key.parse()?));

    // Create the token account, fund it, deploy the token contract and mint the supply to Alice
    let wasm = build_contract().await?;
    let contract_id = alice.account_id().sub_account("lockable-ft")?;

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .add_action(
            FunctionCall::new("new")
                .args(json!({
                    "owner_id": alice.account_id(),
                    "total_supply": U128(TOTAL_SUPPLY),
                }))
                .gas(Gas::from_tgas(1)),
        )
        .await?
        .result()?;

    Ok((node, alice, bob, contract_id))
}

/// The tokens `owner_id` can spend, not counting the locked ones.
pub async fn unlocked_balance(
    near: &Near,
    contract_id: &AccountId,
    owner_id: &AccountId,
) -> anyhow::Result<u128> {
    let balance: U128 = near
        .view(contract_id, "get_unlocked_balance")
        .args(json!({ "owner_id": owner_id }))
        .await?;

    Ok(balance.0)
}

/// The tokens of `owner_id` that `escrow_account_id` may still lock or transfer.
pub async fn allowance(
    near: &Near,
    contract_id: &AccountId,
    owner_id: &AccountId,
    escrow_account_id: &AccountId,
) -> anyhow::Result<u128> {
    let allowance: U128 = near
        .view(contract_id, "get_allowance")
        .args(json!({
            "owner_id": owner_id,
            "escrow_account_id": escrow_account_id,
        }))
        .await?;

    Ok(allowance.0)
}

/// The tokens of `owner_id` that `escrow_account_id` has locked.
pub async fn locked_balance(
    near: &Near,
    contract_id: &AccountId,
    owner_id: &AccountId,
    escrow_account_id: &AccountId,
) -> anyhow::Result<u128> {
    let balance: U128 = near
        .view(contract_id, "get_locked_balance")
        .args(json!({
            "owner_id": owner_id,
            "escrow_account_id": escrow_account_id,
        }))
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
