#[allow(dead_code)]
use fungible_token::FungibleToken;
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use near_sdk_sim::lazy_static;
use near_sdk_sim::test_runtime::{init_test_runtime, to_yocto, TestRuntime};

lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}

#[cfg(feature = "simulation")]
fn setup_multi_token_pool() -> (TestRuntime, FungibleToken, String, String, String) {
    let mut runtime = init_test_runtime();
    let root = "root".to_string();
    let user1 = "user1".to_string();
    let user_1 = runtime.create_user(root.clone(), user1.clone(), to_yocto("100000"));
    let token = FungibleToken::new(
        "root".to_string(),
        U128::from(to_yocto("10000000")),
        &mut runtime,
        &user1,
        &"10000000".to_string(),
        &TOKEN_WASM_BYTES,
    );
    let user2 = "user2".to_string();
    let user_2 = runtime.create_user(root.clone(), user2.clone(), to_yocto("100000"));

    (runtime, token, root, user1, user2)
}

#[cfg(feature = "simulation")]
#[test]
fn simple() {
    let (mut runtime, token, root, user1, user2) = setup_multi_token_pool();
    let expected: U128 = to_yocto("10000000").into();
    assert_eq!(expected, token.get_total_supply(&mut runtime));
}
