use std::str::FromStr;

use near_workspaces::cargo_near_build;
use near_workspaces::types::{AccountId, NearToken};
use test_case::test_case;

async fn build_contract(path: &str) -> anyhow::Result<Vec<u8>> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let manifest = cargo_near_build::camino::Utf8PathBuf::from_str(&path)
            .map_err(|e| anyhow::anyhow!("camino: {e}"))?
            .join("Cargo.toml");
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

#[test_case("./high-level")]
#[test_case("./low-level")]
#[tokio::test]
async fn test_deploy_status_message(contract_path: &str) -> anyhow::Result<()> {
    let wasm = build_contract(contract_path).await?;
    let worker = near_workspaces::sandbox_with_version("2.13.0-rc.2").await?;
    let contract = worker.dev_deploy(&wasm).await?;

    let status_id: AccountId = format!("status.{}", contract.id()).parse()?;
    let status_amt = NearToken::from_near(20);
    let res = contract
        .call("deploy_status_message")
        .args_json((&status_id, status_amt))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    assert!(res.is_success());

    let message = "hello world from factory";
    let res =
        contract.call("complex_call").args_json((status_id, message)).max_gas().transact().await?;
    assert!(res.is_success());
    let value = res.json::<String>()?;
    assert_eq!(message, value.trim_matches(|c| c == '"'));

    Ok(())
}
