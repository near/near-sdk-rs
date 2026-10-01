use digest::{
    Output, OutputSizeUser,
    consts::{U32, U48, U64},
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

pub type Sha3_256 = DigestFn<Sha3_256Fn>;
pub struct Sha3_256Fn;

impl OutputSizeUser for Sha3_256Fn {
    type OutputSize = U32;
}

impl DigestFinalizer for Sha3_256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha3_256(bytes).into()
    }
}

pub type Sha3_384 = DigestFn<Sha3_384Fn>;
pub struct Sha3_384Fn;

impl OutputSizeUser for Sha3_384Fn {
    type OutputSize = U48;
}

impl DigestFinalizer for Sha3_384Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha3_384(bytes).into()
    }
}

pub type Sha3_512 = DigestFn<Sha3_512Fn>;
pub struct Sha3_512Fn;

impl OutputSizeUser for Sha3_512Fn {
    type OutputSize = U64;
}

impl DigestFinalizer for Sha3_512Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha3_512(bytes).into()
    }
}
