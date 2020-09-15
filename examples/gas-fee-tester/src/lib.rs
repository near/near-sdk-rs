use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;
use near_sdk::serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
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

    // Vec of vecs

    pub fn input_json_vec_vec_u8_v(v: Vec<Vec<u8>>) {}

    pub fn input_borsh_vec_vec_u8_v(#[serializer(borsh)] v: Vec<Vec<u8>>) {}

    pub fn output_json_vec_vec_u8_v(v: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        v
    }

    #[result_serializer(borsh)]
    pub fn output_borsh_vec_vec_u8_v(#[serializer(borsh)] v: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        v
    }

    // Vec of strings

    pub fn input_json_vec_string_v(v: Vec<String>) {}

    pub fn input_borsh_vec_string_v(#[serializer(borsh)] v: Vec<String>) {}

    pub fn output_json_vec_string_v(v: Vec<String>) -> Vec<String> {
        v
    }

    #[result_serializer(borsh)]
    pub fn output_borsh_vec_string_v(#[serializer(borsh)] v: Vec<String>) -> Vec<String> {
        v
    }
}
