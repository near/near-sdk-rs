use digest::{
    Output, OutputSizeUser,
    consts::{U32, U64},
};

use crate::utils::{DigestFinalizer, DigestFn};

pub type Sha256 = DigestFn<Sha256Fn>;
pub struct Sha256Fn;

pub type Sha512 = DigestFn<Sha512Fn>;
pub struct Sha512Fn;

impl OutputSizeUser for Sha256Fn {
    type OutputSize = U32;
}

impl DigestFinalizer for Sha256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha256_array(bytes).into()
    }
}

impl OutputSizeUser for Sha512Fn {
    type OutputSize = U64;
}

impl DigestFinalizer for Sha512Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha512_array(bytes).into()
    }
}
