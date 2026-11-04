//! A contract imports only the `promise_batch_action_*` host functions of the actions it
//! builds. Deploying a contract that imports a host function the network doesn't have yet
//! fails, even if the function is never called.

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn promise_imports_only_the_actions_it_builds() -> anyhow::Result<()> {
    let wasm = common::build_test_contract("promise_transfer").await?;

    let mut actions = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&wasm) {
        if let wasmparser::Payload::ImportSection(reader) = payload? {
            for import in reader {
                let name = import?.name;
                if name.starts_with("promise_batch_action_") {
                    actions.push(name.to_owned());
                }
            }
        }
    }

    assert_eq!(actions, ["promise_batch_action_transfer"]);
    Ok(())
}
