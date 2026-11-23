use std::time::Duration;

use near_kit::protocol::Action;
use near_kit::rpc::{FinalExecutionOutcome, RpcError};
use near_kit::signer::InMemorySigner;
use near_kit::transaction::{Final, FunctionCall, Included};
use near_kit::{AccountId, Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde::Deserialize;
use serde_json::json;
use tokio::time::{Instant, timeout_at};

/// The signature the MPC server submits for Alice's request.
const SIGNATURE_TEXT: &str = "I'm a cool signature, Kolo?";

/// How long `sign` waits for Alice's transaction to become final.
const FINAL_STATUS_TIMEOUT: Duration = Duration::from_secs(120);

/// A pending request as `get_requests` returns it. The tests only need its data id.
#[derive(Debug, Deserialize)]
struct SignatureRequest {
    data_id: [u8; 32],
}

/// Builds this crate's contract with `cargo near` and returns the wasm.
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

/// Starts a node with Alice and the MPC server in genesis and deploys the contract to
/// `mpc.alice`.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
async fn setup() -> anyhow::Result<(Sandbox, SigningNear, SigningNear, AccountId)> {
    let alice_account = GenesisAccount::default_with_name(account!("alice"));
    let mpc_server_account = GenesisAccount::default_with_name(account!("mpc-server"));

    let node = Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![alice_account.clone(), mpc_server_account.clone()],
        ..Default::default()
    })
    .await?;

    let client = Near::custom(&node.rpc_addr, "sandbox").build();

    let alice = client.with_signer(InMemorySigner::new(
        alice_account.account_id,
        alice_account.private_key.parse()?,
    ));
    let mpc_server = client.with_signer(InMemorySigner::new(
        mpc_server_account.account_id,
        mpc_server_account.private_key.parse()?,
    ));

    // Create the contract account, fund it and deploy the contract
    let wasm = build_contract().await?;
    let contract_id = alice.account_id().sub_account("mpc")?;

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    Ok((node, alice, mpc_server, contract_id))
}

/// Alice asks the contract to sign a message. Waits until her transaction and every receipt it
/// spawned are final, including the callbacks that run once the request gets a signature or
/// times out.
async fn sign(
    alice: &SigningNear,
    contract_id: &AccountId,
) -> anyhow::Result<FinalExecutionOutcome> {
    let submitted = alice
        .transaction(contract_id)
        .add_action(
            FunctionCall::new("sign")
                .args(json!({ "message": "Hello, world!" }))
                .gas(Gas::from_tgas(14)),
        )
        .wait_until::<Included>()
        .await?;

    // The transaction stays open until the MPC server responds or the request times out, which
    // can take longer than the node waits to answer one request. Ask again after each timeout,
    // but fail the test if the transaction still isn't final by the deadline
    let deadline = Instant::now() + FINAL_STATUS_TIMEOUT;

    loop {
        let tx_status = alice
            .near()
            .tx_status(&submitted.transaction_hash, alice.account_id().clone())
            .wait_until::<Final>();

        let Ok(tx_details) = timeout_at(deadline, tx_status).await else {
            anyhow::bail!(
                "transaction {} isn't final after {FINAL_STATUS_TIMEOUT:?}",
                submitted.transaction_hash
            );
        };

        match tx_details {
            Err(error) if matches!(error.as_rpc(), Some(RpcError::Timeout { .. })) => continue,
            tx_details => return Ok(tx_details?),
        }
    }
}

/// The MPC server waits for Alice's request to show up in the contract and responds to it with
/// a signature.
async fn respond(mpc_server: &SigningNear, contract_id: &AccountId) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    for _ in 0..10 {
        interval.tick().await;

        let requests: Vec<SignatureRequest> =
            mpc_server.near().view(contract_id, "get_requests").await?;

        if let Some(request) = requests.first() {
            mpc_server
                .transaction(contract_id)
                .add_action(
                    FunctionCall::new("sign_respond")
                        .args(json!({
                            "data_id": request.data_id,
                            "signature": SIGNATURE_TEXT,
                        }))
                        .gas(Gas::from_tgas(2)),
                )
                .await?
                .result()?;

            return Ok(());
        }
    }

    anyhow::bail!("no signature request showed up in the contract")
}

#[tokio::test]
async fn happy_path() -> anyhow::Result<()> {
    let (_node, alice, mpc_server, contract_id) = setup().await?;

    // Alice starts the signing process. The MPC server is some service that waits for her
    // request and responds to it
    let (tx_details, ()) =
        tokio::try_join!(sign(&alice, &contract_id), respond(&mpc_server, &contract_id))?;

    // For Alice it looks like a single transaction, but in reality it waits for an outside
    // entity to respond before it continues
    assert_eq!(
        tx_details.logs().collect::<Vec<_>>(),
        [format!("fn do_something invoked with result 'signature received: {SIGNATURE_TEXT}'")]
    );

    Ok(())
}

#[tokio::test]
async fn negative_path() -> anyhow::Result<()> {
    let (_node, alice, _, contract_id) = setup().await?;

    // Alice starts the signing process, but the MPC server doesn't respond, so the request
    // times out after `yield_timeout_length_in_blocks` blocks, a protocol-level parameter
    let tx_details = sign(&alice, &contract_id).await?;

    tx_details.result()?;
    assert_eq!(
        tx_details.logs().collect::<Vec<_>>(),
        ["fn do_something invoked with result 'signature request timed out'"]
    );

    Ok(())
}
