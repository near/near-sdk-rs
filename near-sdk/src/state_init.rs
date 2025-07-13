use near_account_id::AccountId;
use near_sdk_macros::near;
use near_token::NearToken;

use crate::{env, json_types::Base64VecU8};

/// Initialization state for non-existing contract
#[near(inside_nearsdk, serializers=[borsh, json])]
pub struct StateInit {
    /// Code to deploy
    pub code: ContractCode,
    /// Optional funcion call to perform on first initialization
    pub init_call: Option<StateInitFunctionCall>,
}

impl StateInit {
    /// Returns deterministically derived `AccountId`.
    ///
    /// We reuse existing implicit eth addresses and add custom prefix to
    /// second prehash to ensure we avoid collisions between secp256k1
    /// public keys and [`StateInit`] borsh representation.
    /// So, the final schema looks like:
    /// "0x" .. hex(keccak256("state_init" .. keccak256(state_init))[12..32])
    pub fn derived_account_id(&self) -> AccountId {
        let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
        let hash = env::keccak256_array(
            &[b"state_init".as_slice(), &env::keccak256_array(&serialized)].concat(),
        );

        format!("0x{}", hex::encode(&hash[12..32])).parse().unwrap_or_else(|_| unreachable!())
    }
}

/// Code to deploy for non-existing contract
#[near(inside_nearsdk, serializers=[borsh, json])]
#[serde(tag = "location", content = "data", rename_all = "snake_case")]
pub enum ContractCode {
    /// Actual WASM binary
    Inline(Vec<u8>),
    /// Reference global contract's code by its [`AccountId`]
    RefGlobalAccountId(AccountId),
    /// Reference global contract's code by its hash
    RefGlobalHash([u8; 32]),
}

/// Function call arguments for first initialization
#[near(inside_nearsdk, serializers=[borsh, json])]
pub struct StateInitFunctionCall {
    pub function_name: String,
    pub args: Base64VecU8,
}

/// Arguments for [`.function_call_weight_state_init()`](crate::Promise::function_call_weight_state_init)
#[near(inside_nearsdk, serializers=[borsh, json])]
pub struct StateInitArgs {
    pub state_init: StateInit,
    pub amount: NearToken,
    pub refund_to: AccountId,
}
