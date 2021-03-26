use defi::DeFiContract;
use fungible_token::ContractContract as FtContract;

use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk_sim::{
    deploy, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT,
};

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    FT_WASM_BYTES => "res/fungible_token.wasm",
    DEFI_WASM_BYTES => "res/defi.wasm",
}

const FT_ID: &str = "ft";
const DEFI_ID: &str = "defi";

// Register the given `user` with FT contract
pub fn register_user(user: &near_sdk_sim::UserAccount) {
    user.call(
        FT_ID.to_string(),
        "storage_deposit",
        &json!({
            "account_id": user.valid_account_id()
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 125, // attached deposit
    )
    .assert_success();
}

pub fn init_no_macros(initial_balance: u128) -> (UserAccount, UserAccount, UserAccount) {
    let root = init_simulator(None);

    let ft = root.deploy(&FT_WASM_BYTES, FT_ID.into(), STORAGE_AMOUNT);

    ft.call(
        FT_ID.into(),
        "new_default_meta",
        &json!({
            "owner_id": root.valid_account_id(),
            "total_supply": U128::from(initial_balance),
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS / 2,
        0, // attached deposit
    )
    .assert_success();

    let alice = root.create_user("alice".to_string(), to_yocto("100"));
    register_user(&alice);

    (root, ft, alice)
}

pub fn init_with_macros(
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
        bytes: &FT_WASM_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method: new_default_meta(
            root.valid_account_id(),
            initial_balance.into()
        )
    );
    let alice = root.create_user("alice".to_string(), to_yocto("100"));
    register_user(&alice);

    let defi = deploy!(
        contract: DeFiContract,
        contract_id: DEFI_ID,
        bytes: &DEFI_WASM_BYTES,
        signer_account: root,
        init_method: new(
            ft.valid_account_id()
        )
    );

    (root, ft, defi, alice)
}
