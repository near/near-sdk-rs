pub use lazy_static::*;
pub mod outcome;
pub use outcome::*;
pub mod runtime;
pub mod units;
pub mod user;
pub use near_primitives::*;
pub use units::*;
pub use user::*;

#[cfg(doctest)]
lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../../examples/fungible-token/res/fungible_token.wasm").as_ref();
}

#[cfg(doctest)]
pub fn get_wasm_bytes() -> &'static [u8] {
    &TOKEN_WASM_BYTES
}

#[cfg(doctest)]
pub fn test_setup() {
    let master_account = crate::init_simulator(None);
    use fungible_token::FungibleTokenContract;
    let contract = crate::deploy! {
        contract: FungibleTokenContract,
          contract_id: "contract",
          bytes: &TOKEN_WASM_BYTES,
          signer_account: master_account,
          deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
          gas: near_sdk_sim::DEFAULT_GAS,
          init_method: new(master_account.account_id, initial_balance.into())
    };
}
