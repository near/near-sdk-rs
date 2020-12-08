use near_sdk_sim::{
    call, deploy, init_simulator, near_crypto::Signer, to_yocto, ContractAccount,
    UserAccount,
};
use near_sdk::serde::{Deserialize};
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::io::Write;

/// Bring contract crate into namespace
extern crate collection_comparison;
/// Import the generated proxy contract
/// Magic function??
use collection_comparison::CollectionsContract;
use near_sdk_sim::account::AccessKey;

near_sdk_sim::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/collection_comparison.wasm").as_ref();
}

fn init() -> (UserAccount, ContractAccount<CollectionsContract>, UserAccount) {
    // todo: useful comment here
    let master_account = init_simulator(None);
    // uses default values for deposit and gas
    let contract_user = deploy!(
        // Contract Proxy
        contract: CollectionsContract,
        // Contract account id
        contract_id: "contract",
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: master_account,
        // init method
        init_method: new()
    );
    let alice = master_account.create_user("alice".to_string(), to_yocto("100"));
    (master_account, contract_user, alice)
}

/// Example of how to create and use an user transaction.
fn init2(initial_balance: u128) {
    let master_account = init_simulator(None);
    let txn = master_account.create_transaction("contract".into());
    // uses default values for deposit and gas
    let res = txn
        .create_account()
        .add_key(master_account.signer.public_key(), AccessKey::full_access())
        .transfer(initial_balance)
        .deploy_contract((&TOKEN_WASM_BYTES).to_vec())
        .submit();
    println!("{:#?}", res);
}

#[test]
pub fn mint_token() {
    init2(to_yocto("35"));
}

#[test]
fn test_add_view_gas() {
    let file = File::open("src/data/alpha-sha.csv").unwrap();
    let reader = BufReader::new(file);

    let (master_account, contract, _) = init();

    let mut file_adding = File::create("src/data/output-adding.json").unwrap();
    let mut file_reading = File::create("src/data/output-reading.json").unwrap();
    writeln!(&mut file_adding, "[").unwrap();
    writeln!(&mut file_reading, "[").unwrap();
    for (i, line) in reader.lines().enumerate() {
        if i != 0 {
            write!(&mut file_adding, ",\n").unwrap(); // todo do i need unwrap?
            write!(&mut file_reading, ",\n").unwrap(); // todo do i need unwrap?
        }
        let line = line.unwrap();
        let split = line.split(",");
        let vec: Vec<&str> = split.collect();
        let key = vec.get(0).unwrap().to_string();
        let val = vec.get(1).unwrap().to_string();
        println!("key: {:#?}, value: {:#?}", key, val);

        let call_result = call!(
            master_account,
            contract.add_tree_map(key.clone(), val)
        );
        println!("gas used adding key \t\t{:#?}", call_result.gas_burnt());
        let view_result = call!(
            master_account,
            contract.get_tree_map(key)
        );
        println!("gas used retrieving key \t{:#?}", view_result.gas_burnt());
        write!(&mut file_adding, "  {}", call_result.gas_burnt()).unwrap();
        write!(&mut file_reading, "  {}", view_result.gas_burnt()).unwrap();
    }
    writeln!(&mut file_adding, "\n]").unwrap();
    writeln!(&mut file_reading, "\n]").unwrap();
}
