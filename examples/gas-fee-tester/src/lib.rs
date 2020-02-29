use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct A {
    a: u32,
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct GasFeeTester {}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn global_noop() {}

#[allow(unused_variables)]
#[near_bindgen]
impl GasFeeTester {
    pub fn structure_noop() {}

    // Integers

    pub fn input_json_u32_a(a: u32) {}

    pub fn input_json_u32_aa(aa: u32) {}

    pub fn output_json_u32_a(a: u32) -> u32 {
        a
    }

    pub fn input_borsh_u32_a(#[serializer(borsh)] a: u32) {}

    #[result_serializer(borsh)]
    pub fn output_borsh_u32_a(#[serializer(borsh)] a: u32) -> u32 {
        a
    }

    pub fn input_json_u32_ab(a: u32, b: u32) {}

    pub fn input_borsh_u32_ab(#[serializer(borsh)] a: u32, #[serializer(borsh)] b: u32) {}

    // Strings

    pub fn input_json_string_s(s: String) {}

    pub fn input_borsh_string_s(#[serializer(borsh)] s: String) {}

    pub fn output_json_string_s(s: String) -> String {
        s
    }

    #[result_serializer(borsh)]
    pub fn output_borsh_string_s(#[serializer(borsh)] s: String) -> String {
        s
    }

    // Vec<u8>

    pub fn input_json_vec_u8_v(v: Vec<u8>) {}

    pub fn input_borsh_vec_u8_v(#[serializer(borsh)] v: Vec<u8>) {}

    pub fn output_json_vec_u8_v(v: Vec<u8>) -> Vec<u8> {
        v
    }

    #[result_serializer(borsh)]
    pub fn output_borsh_vec_u8_v(#[serializer(borsh)] v: Vec<u8>) -> Vec<u8> {
        v
    }

    // Vec<u32>

    pub fn input_json_vec_u32_v(v: Vec<u32>) {}

    pub fn input_borsh_vec_u32_v(#[serializer(borsh)] v: Vec<u32>) {}

    pub fn output_json_vec_u32_v(v: Vec<u32>) -> Vec<u32> {
        v
    }

    #[result_serializer(borsh)]
    pub fn output_borsh_vec_u32_v(#[serializer(borsh)] v: Vec<u32>) -> Vec<u32> {
        v
    }

    // Simple Struct

    pub fn input_json_struct_a(a: A) {}

    pub fn input_borsh_struct_a(#[serializer(borsh)] a: A) {}

    pub fn output_json_struct_a(a: A) -> A {
        a
    }

    #[result_serializer(borsh)]
    pub fn output_borsh_struct_a(#[serializer(borsh)] a: A) -> A {
        a
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_bindgen::MockedBlockchain;
    use near_bindgen::{testing_env, VMContext};

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
        }
    }

    #[test]
    fn set_get_message() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = StatusMessage::default();
        contract.set_status("hello".to_string());
        assert_eq!("hello".to_string(), contract.get_status("bob_near".to_string()).unwrap());
    }

    #[test]
    fn get_nonexistent_message() {
        let context = get_context(vec![], true);
        testing_env!(context);
        let contract = StatusMessage::default();
        assert_eq!(None, contract.get_status("francis.near".to_string()));
    }
}
