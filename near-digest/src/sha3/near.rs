use digest::{
    Output, OutputSizeUser,
    consts::{U32, U64},
};

use crate::utils::{DigestFinalizer, DigestFn};

pub type Keccak256 = DigestFn<Keccak256Fn>;
pub struct Keccak256Fn;

impl OutputSizeUser for Keccak256Fn {
    type OutputSize = U32;
}

impl DigestFinalizer for Keccak256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::keccak256_array(bytes).into()
    }
}

pub type Keccak512 = DigestFn<Keccak512Fn>;
pub struct Keccak512Fn;

impl OutputSizeUser for Keccak512Fn {
    type OutputSize = U64;
}

impl DigestFinalizer for Keccak512Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::keccak512_array(bytes).into()
    }
}
