use near_primitives::views::FinalExecutionStatus;
use near_units::parse_near;
use workspaces::prelude::*;
use workspaces::{Account, AccountId, Contract, DevNetwork, Network, Worker};
use near_contract_standards::multi_token::{
    metadata::{TokenMetadata},
};
use near_contract_standards::multi_token::token::{Token, TokenId};
use near_sdk::{Balance};

pub async fn register_user_for_token(
    worker: &Worker<impl Network>,
    contract: &Contract,
    account_id: &AccountId,
    token_id: TokenId,
) -> anyhow::Result<()> {
    let res = contract
        .call(worker, "register")
        .args_json((token_id.clone(), account_id))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    Ok(())
}

pub async fn helper_mint(
    mt_contract: &Contract,
    worker: &Worker<impl DevNetwork>,
    owner_id: AccountId,
    amount: Balance,
    title: String,
    desc: String,
) -> anyhow::Result<Token> {
    let token_md: TokenMetadata = TokenMetadata {
        title: Some(title),
        description: Some(desc),
        media: None,
        media_hash: None,
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    };

    let res = mt_contract
        .call(worker, "mt_mint")
        .args_json((owner_id, token_md, amount))?
        .gas(300_000_000_000_000)
        .deposit(parse_near!("7 mN"))
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));
    let token: Token = res.json()?;

    Ok(token)
}


// Returns Multi-token contract, a non-owner user Alice, and a DeFi contract
// for receiving cross-contract calls.
pub async fn init(
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<(Contract, Account, Contract)> {
    let mt_contract = worker.dev_deploy(include_bytes!("../../res/multi_token.wasm").to_vec()).await?;

    let res = mt_contract
        .call(worker, "new_default_meta")
        .args_json((mt_contract.id(),))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let defi_contract = worker.dev_deploy(include_bytes!("../../res/defi.wasm").to_vec()).await?;

    let res = defi_contract
        .call(worker, "new")
        .args_json((mt_contract.id(),))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let alice = mt_contract
        .as_account()
        .create_subaccount(worker, "alice")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    Ok((mt_contract, alice, defi_contract))
}

pub async fn init_approval_receiver_contract(worker: &Worker<impl DevNetwork>) -> anyhow::Result<Contract> {
    let approval_receiver_contract = worker.dev_deploy(include_bytes!("../../res/approval_receiver.wasm").to_vec()).await?;
    let res = approval_receiver_contract
        .call(worker, "new")
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    Ok(approval_receiver_contract)
}
