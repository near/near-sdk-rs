use near_kit::protocol::Action;
use near_kit::signer::InMemorySigner;
use near_kit::transaction::FunctionCall;
use near_kit::{Gas, Near, NearToken, account};
use near_sandbox::{GenesisAccount, Sandbox, SandboxConfig};

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

#[tokio::test]
async fn test_add_callback_vec() -> anyhow::Result<()> {
    // Alice exists from genesis. She pays for the setup and calls the contract.
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
    let contract_id = alice.account_id().sub_account("adder")?;

    alice
        .transaction(&contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .await?
        .result()?;

    let sum: (u32, u32) = alice
        .transaction(&contract_id)
        .add_action(FunctionCall::new("call_all").gas(Gas::from_tgas(10)))
        .await?
        .json()?;

    assert_eq!(sum, (21, 33));

    Ok(())
}
