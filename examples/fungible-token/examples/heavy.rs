use near_sdk::AccountId;
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, ContractAccount, UserAccount};

/// Bring contract crate into namespace
extern crate fungible_token;
/// Import the generated proxy contract
use fungible_token::FungibleTokenContract;

near_sdk_sim::lazy_static! {
  /// Load in contract bytes
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<FungibleTokenContract>, UserAccount) {
    let master_account = init_simulator(None);
    // uses default values for deposit and gas
    let contract_user = deploy!(
        // Contract Proxy
        contract: FungibleTokenContract,
        // Contract account id
        contract_id: "contract",
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: master_account,
        // init method
        init_method: new(master_account.account_id(), initial_balance.into())
    );
    let alice = master_account.create_user("alice".to_string(), to_yocto("100"));
    (master_account, contract_user, alice)
}

const TRANSFER_DEPOSIT: u128 = 13300000000000000000000u128;

fn transfer(
    from: &UserAccount,
    to: AccountId,
    amount: u128,
    contract: &ContractAccount<FungibleTokenContract>,
) {
    call!(from, contract.transfer(to, amount.into()), deposit = TRANSFER_DEPOSIT).assert_success()
}

fn main() {
    let iterations: u64 = if std::env::args().len() >= 2 {
        (&std::env::args().collect::<Vec<String>>()[1]).parse().unwrap()
    } else {
        10
    };
    near_sdk_sim::enable();
    let transfer_amount = to_yocto("1");
    let initial_balance = to_yocto("10000000000");
    let (master_account, contract, alice) = init(initial_balance);
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
