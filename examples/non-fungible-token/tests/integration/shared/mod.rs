use std::collections::BTreeMap;

use near_kit::protocol::{Action, ActionErrorKind, FunctionCallError};
use near_kit::rpc::{FinalExecutionOutcome, Finality, STORAGE_AMOUNT_PER_BYTE};
use near_kit::signer::InMemorySigner;
use near_kit::standards::NftToken;
use near_kit::transaction::{ExecutedOptimistic, FunctionCall};
use near_kit::{AccountId, Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

/// The token Alice mints to herself in setup.
pub const TOKEN_ID: &str = "0";

/// The deposit `nft_transfer`, `nft_transfer_call`, `nft_revoke` and `nft_revoke_all` require.
pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

/// The data of an `nft_transfer` event.
#[derive(Debug, PartialEq, Deserialize)]
pub struct NftTransfer {
    pub old_owner_id: AccountId,
    pub new_owner_id: AccountId,
    pub token_ids: Vec<String>,
    pub authorized_id: Option<AccountId>,
    pub memo: Option<String>,
}

impl NftTransfer {
    /// `TOKEN_ID` going from `old_owner_id` to `new_owner_id`, transferred by its owner.
    pub fn new(old_owner_id: &AccountId, new_owner_id: &AccountId, memo: Option<&str>) -> Self {
        Self {
            old_owner_id: old_owner_id.clone(),
            new_owner_id: new_owner_id.clone(),
            token_ids: vec![TOKEN_ID.to_string()],
            authorized_id: None,
            memo: memo.map(str::to_string),
        }
    }
}

/// The NEP-171 events the tests read, as near-contract-standards logs them.
#[derive(Debug, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
enum NftEvent {
    NftTransfer(Vec<NftTransfer>),
}

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

/// Mints `token_id` to `owner`, who owns the token contract: only they can mint.
pub async fn mint(
    owner: &SigningNear<ExecutedOptimistic>,
    nft_contract_id: &AccountId,
    token_id: &str,
    title: &str,
    description: &str,
) -> anyhow::Result<()> {
    owner
        .transaction(nft_contract_id)
        .add_action(
            FunctionCall::new("nft_mint")
                .args(json!({
                    "token_id": token_id,
                    "token_owner_id": owner.account_id(),
                    "token_metadata": {
                        "title": title,
                        "description": description,
                        "copies": 1,
                    },
                }))
                .gas(Gas::from_tgas(5))
                // More than the token's storage costs. The token contract refunds the rest.
                .deposit(NearToken::from_millinear(7)),
        )
        .await?
        .result()?;

    Ok(())
}

/// Starts a node with Alice and Bob in genesis and deploys the token contract to `nft.alice`.
/// Alice owns the contract and the token `TOKEN_ID`. Bob owns no tokens.
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

    // Create the token account, fund it, deploy the token contract and make Alice its owner
    let wasm = build_contract("nft").await?;
    let nft_contract_id = alice.account_id().sub_account("nft")?;

    alice
        .transaction(&nft_contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .add_action(
            FunctionCall::new("new_default_meta")
                .args(json!({ "owner_id": alice.account_id() }))
                .gas(Gas::from_tgas(4)),
        )
        .await?
        .result()?;

    mint(
        &alice,
        &nft_contract_id,
        TOKEN_ID,
        "Olympus Mons",
        "The tallest mountain in the charted solar system",
    )
    .await?;

    Ok((node, alice, bob, nft_contract_id))
}

/// Deploys the receiver contract in `path` (relative to this crate) to `<name>.alice`.
/// The receiver accepts calls from the token contract only.
pub async fn deploy_receiver(
    alice: &SigningNear<ExecutedOptimistic>,
    nft_contract_id: &AccountId,
    path: &str,
    name: &str,
) -> anyhow::Result<AccountId> {
    let wasm = build_contract(path).await?;
    let receiver_id = alice.account_id().sub_account(name)?;

    alice
        .transaction(&receiver_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .add_action(
            FunctionCall::new("new")
                .args(json!({ "non_fungible_token_account_id": nft_contract_id }))
                .gas(Gas::from_tgas(1)),
        )
        .await?
        .result()?;

    Ok(receiver_id)
}

/// What the token contract charges to store an approval for `account_id`: the account id
/// with its 4-byte length, and the 8-byte approval id.
pub fn approval_deposit(account_id: &AccountId) -> NearToken {
    STORAGE_AMOUNT_PER_BYTE.saturating_mul(account_id.len() as u128 + 12)
}

/// `nft_approve` that approves `account_id` for `TOKEN_ID` and pays for storing the approval.
/// It passes no `msg`, so the token contract doesn't call `account_id`.
pub fn approve(account_id: &AccountId) -> FunctionCall {
    FunctionCall::new("nft_approve")
        .args(json!({
            "token_id": TOKEN_ID,
            "account_id": account_id,
        }))
        .gas(Gas::from_tgas(2))
        .deposit(approval_deposit(account_id))
}

/// The token `TOKEN_ID`.
pub async fn nft_token(near: &Near, nft_contract_id: &AccountId) -> anyhow::Result<NftToken> {
    let token: Option<NftToken> = near
        .view(nft_contract_id, "nft_token")
        .args(json!({ "token_id": TOKEN_ID }))
        .at(Finality::Optimistic)
        .await?;

    Ok(token.expect("setup mints the token"))
}

/// Whether `account_id` is approved for `TOKEN_ID`. With `approval_id`, the approval must
/// also have that id.
pub async fn is_approved(
    near: &Near,
    nft_contract_id: &AccountId,
    account_id: &AccountId,
    approval_id: Option<u64>,
) -> anyhow::Result<bool> {
    Ok(near
        .view(nft_contract_id, "nft_is_approved")
        .args(json!({
            "token_id": TOKEN_ID,
            "approved_account_id": account_id,
            "approval_id": approval_id,
        }))
        .at(Finality::Optimistic)
        .await?)
}

/// Every `nft_transfer` event the token contract logged in receipts that succeeded.
pub fn nft_transfers(
    tx_details: &FinalExecutionOutcome,
    nft_contract_id: &AccountId,
) -> anyhow::Result<Vec<NftTransfer>> {
    let mut transfers = Vec::new();

    for event in tx_details.events(nft_contract_id) {
        let event = event?.decode::<NftEvent>()?;

        anyhow::ensure!(event.standard == "nep171", "not a NEP-171 event: {event:?}");

        let NftEvent::NftTransfer(data) = event.payload;
        transfers.extend(data);
    }

    Ok(transfers)
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
