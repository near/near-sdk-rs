use near_units::parse_near;
use workspaces::{Account, AccountId, Contract, DevNetwork, Worker};
use near_contract_standards::multi_token::{
    metadata::{TokenMetadata},
};
use near_contract_standards::multi_token::token::{Token};
use near_sdk::{Balance};
use near_contract_standards::storage_management::StorageBalanceBounds;

pub async fn get_storage_balance_bounds(contract: &Contract) -> anyhow::Result<StorageBalanceBounds> {
    Ok(contract.view("storage_balance_bounds", vec![])
        .await?
        .json::<StorageBalanceBounds>()?)
}

pub async fn register_user_for_token(
    contract: &Contract,
    account_id: &AccountId,
    deposit: u128,
) -> anyhow::Result<()> {
    let res = contract.call("storage_deposit")
        .args_json((
            account_id,
            Some(false),
        ))
        .max_gas()
        .deposit(deposit)
        .transact()
        .await?;
    assert!(res.is_success());
    Ok(())
}

pub async fn helper_mint(
    mt_contract: &Contract,
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
        .call("mt_mint")
        .args_json((owner_id, token_md, amount.to_string()))
        .max_gas()
        .deposit(parse_near!("7 mN"))
        .transact()
        .await?;
    assert!(res.is_success());
    let token: Token = res.json()?;

    Ok(token)
}


// Returns Multi-token contract, a non-owner user Alice, and a DeFi contract
// for receiving cross-contract calls.
pub async fn init(
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<(Contract, Account, Account, Contract)> {
    let mt_contract = worker.dev_deploy(include_bytes!("../../res/multi_token.wasm")).await?;

    let res = mt_contract
        .call("new_default_meta")
        .args_json((mt_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    
    assert!(res.is_success());

    let defi_contract = worker.dev_deploy(include_bytes!("../../res/defi.wasm")).await?;

    let res = defi_contract
        .call("new")
        .args_json((mt_contract.id(),))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let alice = mt_contract
        .as_account()
        .create_subaccount("alice")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    let bob = mt_contract
        .as_account()
        .create_subaccount("bob")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    Ok((mt_contract, alice, bob, defi_contract))
}

pub async fn init_approval_receiver_contract(worker: &Worker<impl DevNetwork>) -> anyhow::Result<Contract> {
    let approval_receiver_contract = worker.dev_deploy(include_bytes!("../../res/approval_receiver.wasm")).await?;
    let res = approval_receiver_contract
        .call("new")
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    Ok(approval_receiver_contract)
}
