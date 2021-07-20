use near_sdk::{env, AccountId};
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, ContractAccount, UserAccount};

use defi::*;
/// Import the generated proxy contract
use fungible_token::ContractContract as FtContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
  TOKEN_WASM_BYTES => "res/fungible_token.wasm",
  DEFI_WASM_BYTES => "res/defi.wasm",
}

const _REFERENCE: &str = "https://github.com/near/near-sdk-rs/tree/master/examples/fungible-token";

const FT_ID: &str = "ft";
const DEFI_ID: &str = "defi";

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<FtContract>, ContractAccount<DeFiContract>, UserAccount) {
    let root = init_simulator(None);
    // uses default values for deposit and gas
    let ft = deploy!(
        // Contract Proxy
        contract: FtContract,
        // Contract account id
        contract_id: FT_ID,
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method:
          new_default_meta(
            root.account_id(),
            initial_balance.into()
        )
    );
    let alice = root.create_user(AccountId::new_unchecked("alice".to_string()), to_yocto("100"));
    register_user(&ft, &alice);

    let id = AccountId::new_unchecked(FT_ID.to_string());

    let defi = deploy!(
        contract: DeFiContract,
        contract_id: DEFI_ID,
        bytes: &DEFI_WASM_BYTES,
        signer_account: root,
        init_method: new(id)
    );

    (root, ft, defi, alice)
}

// For given `contract` which uses the Account Storage standard,
// register the given `user`
fn register_user(contract: &ContractAccount<FtContract>, user: &UserAccount) {
    call!(
        user,
        contract.storage_deposit(Some(user.account_id()), None),
        deposit = env::storage_byte_cost() * 125
    )
    .assert_success();
}

const TRANSFER_DEPOSIT: u128 = 1u128;

fn transfer(
    from: &UserAccount,
    to: AccountId,
    amount: u128,
    contract: &ContractAccount<FtContract>,
) {
    call!(from, contract.ft_transfer(to, amount.into(), None), deposit = TRANSFER_DEPOSIT)
        .assert_success()
}

fn main() {
    let iterations: u64 = if std::env::args().len() >= 2 {
        std::env::args().nth(1).unwrap().parse().unwrap()
    } else {
        10
    };
    let transfer_amount = to_yocto("1");
    let initial_balance = to_yocto("10000000000");
    let (master_account, contract, _defi, alice) = init(initial_balance);
    transfer(&master_account, alice.account_id(), transfer_amount, &contract);
    let now = std::time::Instant::now();
    for i in 0..iterations {
        if i % 2 == 0 {
            transfer(&master_account, alice.account_id(), transfer_amount, &contract);
        } else {
            transfer(&alice, master_account.account_id(), transfer_amount, &contract);
        }
    }
    let elapsed = now.elapsed().as_millis();
    println!(
        "{} calls to transfer in {} ms, {} calls / second",
        iterations,
        elapsed,
        (iterations * 1000) as u128 / elapsed
    );
}
