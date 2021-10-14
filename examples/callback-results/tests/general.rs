use near_sdk_sim::{call, deploy, init_simulator, ContractAccount, UserAccount};
extern crate callback_results;
// Note: the struct xxxxxxContract is created by #[near_bindgen] from near-sdk
use callback_results::CallbackContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    CONTRACT_BYTES => "res/callback_results.wasm",
}

fn init() -> (UserAccount, ContractAccount<CallbackContract>) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));
    let contract_account = deploy! {
        contract: CallbackContract,
        contract_id: "contract",
        bytes: &CONTRACT_BYTES,
        signer_account: master_account
    };
    (master_account, contract_account)
}

#[test]
fn callback_sim() {
    let (master_account, contract) = init();

    // Call function a only to ensure it has correct behaviour
    let res = call!(master_account, contract.a());
    assert_eq!(res.unwrap_json::<u8>(), 8);

    // Following tests the function call where the `call_all` function always succeeds and handles
    // the result of the async calls made from within the function with callbacks.

    // No failures
    let res = call!(master_account, contract.call_all(false, 1));
    assert_eq!(res.unwrap_json::<(bool, bool)>(), (false, false));

    // Fail b
    let res = call!(master_account, contract.call_all(true, 1));
    assert_eq!(res.unwrap_json::<(bool, bool)>(), (true, false));

    // Fail c
    let res = call!(master_account, contract.call_all(false, 0));
    assert_eq!(res.unwrap_json::<(bool, bool)>(), (false, true));

    // Fail both b and c
    let res = call!(master_account, contract.call_all(true, 0));
    assert_eq!(res.unwrap_json::<(bool, bool)>(), (true, true));
}
