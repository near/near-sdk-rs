use near_kit::json_types::U128;
use near_kit::protocol::{Action, ActionErrorKind, FunctionCallError};
use near_kit::rpc::{FinalExecutionOutcome, STORAGE_AMOUNT_PER_BYTE};
use near_kit::signer::InMemorySigner;
use near_kit::transaction::FunctionCall;
use near_kit::{AccountId, Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde_json::json;

/// One whole token: the token metadata sets 24 decimals.
pub const ONE_TOKEN: u128 = 10u128.pow(24);

/// The token supply Alice owns after setup.
pub const TOTAL_SUPPLY: u128 = 10_000 * ONE_TOKEN;

/// What the token contract charges to register one account.
pub const STORAGE_DEPOSIT: NearToken = STORAGE_AMOUNT_PER_BYTE.saturating_mul(125);

/// The deposit `ft_transfer`, `ft_transfer_call` and `storage_unregister` require.
pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

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

/// Starts a node with Alice and Bob in genesis and deploys the token contract to `ft.alice`.
/// Alice owns the whole token supply. Bob isn't registered with the token contract.
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
    payer: &SigningNear,
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
