use near_kit::protocol::Action;
use near_kit::signer::InMemorySigner;
use near_kit::transaction::{ExecutedOptimistic, FunctionCall};
use near_kit::{AccountId, Gas, Near, NearToken, SigningNear, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};
use serde_json::json;
use test_case::test_case;

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

/// Starts a node with Alice in genesis and deploys this crate's contract to
/// `callback-results.alice`. Alice pays for the setup and calls the contract.
///
/// The node stops when the returned `Sandbox` is dropped, so keep it until the test ends.
async fn setup() -> anyhow::Result<(Sandbox, SigningNear<ExecutedOptimistic>, AccountId)> {
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

    // Create the contract account, fund it and deploy the contract
    let wasm = build_contract().await?;
    let contract_id = alice.account_id().sub_account("callback-results")?;

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    Ok((node, alice, contract_id))
}

#[tokio::test]
async fn test_a() -> anyhow::Result<()> {
    let (_node, alice, contract_id) = setup().await?;

    // `a` calls `c` with a value that always succeeds.
    let value: u8 = alice
        .transaction(&contract_id)
        .add_action(FunctionCall::new("a").gas(Gas::from_tgas(2)))
        .await?
        .json()?;

    assert_eq!(value, 8);

    Ok(())
}

// `call_all_reverse` takes the callback results in a different argument order
// and must report the same failures as `call_all`.
#[test_case("call_all" ; "call_all")]
#[test_case("call_all_reverse" ; "call_all_reverse")]
#[tokio::test]
async fn test_call_all(method: &str) -> anyhow::Result<()> {
    let (_node, alice, contract_id) = setup().await?;

    // Each case is `(fail_b, c_value, d_value)` and which of `b`, `c` and `d` failed.
    let cases = [
        ((false, 1, 1), (false, false, false)),
        ((true, 1, 1), (true, false, false)),
        ((false, 0, 1), (false, true, false)),
        ((false, 1, 0), (false, false, true)),
        ((true, 0, 1), (true, true, false)),
        ((true, 0, 0), (true, true, true)),
    ];

    for ((fail_b, c_value, d_value), failed) in cases {
        let args = json!({
            "fail_b": fail_b,
            "c_value": c_value,
            "d_value": d_value,
        });

        let reported: (bool, bool, bool) = alice
            .transaction(&contract_id)
            .add_action(FunctionCall::new(method).args(&args).gas(Gas::from_tgas(20)))
            .await?
            .json()?;

        assert_eq!(reported, failed, "{method}({args})");
    }

    Ok(())
}
