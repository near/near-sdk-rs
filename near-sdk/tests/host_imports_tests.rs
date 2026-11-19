//! Every `Promise` sends its actions through one dispatch over all action kinds, so a contract
//! that creates any promise imports the host function of every action the dispatch knows, used
//! or not. Deploying a contract that imports a host function the network does not have yet
//! fails, so actions that need a protocol version above `min_protocol_version` stay behind a
//! cargo feature.

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn promise_without_universal_accounts_does_not_import_universal_state_init()
-> anyhow::Result<()> {
    let wasm = common::build_test_contract("promise_transfer").await?;

    let imports = imported_functions(&wasm)?;

    assert!(imports.iter().any(|name| name == "promise_batch_action_transfer"), "{imports:?}");
    assert!(!imports.iter().any(|name| name.contains("universal_state_init")), "{imports:?}");
    Ok(())
}

fn imported_functions(wasm: &[u8]) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(wasm) {
        if let wasmparser::Payload::ImportSection(reader) = payload? {
            for import in reader {
                names.push(import?.name.to_owned());
            }
        }
    }
    Ok(names)
}
