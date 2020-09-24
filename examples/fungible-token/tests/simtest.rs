// use near_sdk_sim::test_runtime::TxResult;
// use near_sdk::env::attached_deposit;

// near_sdk_sim::lazy_static::lazy_static! {
//     static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
// }

// pub trait Contract<T> {

// }

// pub trait Simulator {
//     fn simulate() -> TxResult;
//     fn createContract<T>(id: String, bytes) -> Contract<T>;

//     fn create() -> Simulator;
// }

// pub struct SimulatorConfig {
//     contracts: [ContractConfig]

// }
// type Users = [User];

// pub fn init_sim() -> (Simulator, User, User) {
//     let simulator =  Simulator.create();
//     let contract = simulator.createAccount(account_id, Some(TOKEN_WASM_BYTES));
//     simulator.deploy(contract).call("new", json!{});
//     let alice = simulator.create_user("alice".into());
//     return (simulator, alice, contract)
// }


// pub fn mint_token() {
//     let (simulator, alice, contract) = init_sim();
//     alice.call(contract, "mint", json!{}, gas, attached_deposit);
//     // let call_transaction = contract.mint(21).options({alice, attached_deposit});
//     // simulator.call_step(call_transaction, alice, attached_deposit());
//     // simulator.call(contract, "mint", json!{}, attached_deposit, alice);
// }
