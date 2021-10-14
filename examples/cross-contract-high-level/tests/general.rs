use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS,
    STORAGE_AMOUNT,
};
extern crate cross_contract_high_level;
// Note: the struct xxxxxxContract is created by #[near_bindgen] from near-sdk in combination with
// near-sdk-sim
use cross_contract_high_level::CrossContractContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TOKEN_WASM_BYTES => "res/cross_contract_high_level.wasm",
}

fn init() -> (UserAccount, ContractAccount<CrossContractContract>) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));
    let contract_account = deploy! {
        contract: CrossContractContract,
        contract_id: "contract",
        bytes: &TOKEN_WASM_BYTES,
        signer_account: master_account
    };
    (master_account, contract_account)
}

#[test]
fn test_sim_transfer() {
    let (master_account, contract) = init();

    let status_id: near_sdk::AccountId = "status".parse().unwrap();
    let status_amt = to_yocto("35");
    call!(
        master_account,
        contract.deploy_status_message(status_id.clone(), status_amt.into()),
        deposit = STORAGE_AMOUNT
    )
    .assert_success();

    let message = "hello world";
    let res = call!(master_account, contract.complex_call(status_id, message.to_string()));
    assert!(res.is_ok(), "complex_call has promise_errors: {:#?}", res.promise_results());

    let value = res.unwrap_json_value();
    assert_eq!(message, value.to_string().trim_matches(|c| c == '"'));
    let v1: Vec<u8> = vec![42];
    let _v: Vec<u8> = vec![7, 1, 6, 5, 9, 255, 100, 11];
    call!(master_account, contract.merge_sort(v1)).assert_success();

    let res = call!(master_account, contract.merge_sort(_v.clone()), gas = DEFAULT_GAS * 500);
    res.assert_success();
    let arr = res.unwrap_borsh::<Vec<u8>>();
    let (_last, b) = arr.iter().fold((0u8, true), |(prev, b), curr| (*curr, prev <= *curr && b));
    assert!(b, "array is not sorted.");
}
