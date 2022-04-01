use near_primitives::views::FinalExecutionStatus;
use near_sdk::json_types::U128;
use near_sdk::ONE_YOCTO;
use near_units::parse_near;
use workspaces::prelude::*;
use workspaces::{Account, AccountId, Contract, DevNetwork, Network, Worker};

async fn register_user(
    worker: &Worker<impl Network>,
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<()> {
    let res = contract
        .call(&worker, "storage_deposit")
        .args_json((account_id, Option::<bool>::None))?
        .gas(300_000_000_000_000)
        .deposit(near_sdk::env::storage_byte_cost() * 125)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    Ok(())
}

async fn init(
    worker: &Worker<impl DevNetwork>,
    initial_balance: U128,
) -> anyhow::Result<(Contract, Account, Contract, Contract)> {
    let mt_contract = worker.dev_deploy(include_bytes!("../res/multi_token.wasm").to_vec()).await?;

    let res = mt_contract
        .call(&worker, "new_default_meta")
        .args_json((mt_contract.id(), initial_balance))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let defi_contract = worker.dev_deploy(include_bytes!("../res/defi.wasm").to_vec()).await?;
    let approval_receiver_contract =
        worker.dev_deploy(include_bytes!("../res/approval_receiver.wasm").to_vec()).await?;

    let res = defi_contract
        .call(&worker, "new")
        .args_json((mt_contract.id(),))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let alice = mt_contract
        .as_account()
        .create_subaccount(&worker, "alice")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;
    register_user(worker, &mt_contract, alice.id()).await?;

    let res = mt_contract
        .call(&worker, "storage_deposit")
        .args_json((alice.id(), Option::<bool>::None))?
        .gas(300_000_000_000_000)
        .deposit(near_sdk::env::storage_byte_cost() * 125)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    return Ok((mt_contract, alice, defi_contract, approval_receiver_contract));
}

#[tokio::test]
async fn test_total_supply() -> anyhow::Result<()> {
    // TODO: Write simulation tests involving cross-contract calls (mt_transfer_call, etc).

    Ok(())
}
