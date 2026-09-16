#[cfg(feature = "unstable")]
use digest::consts::U48;
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

#[cfg(feature = "unstable")]
pub type Sha3_256 = DigestFn<Sha3_256Fn>;
#[cfg(feature = "unstable")]
pub struct Sha3_256Fn;

#[cfg(feature = "unstable")]
impl OutputSizeUser for Sha3_256Fn {
    type OutputSize = U32;
}

#[cfg(feature = "unstable")]
impl DigestFinalizer for Sha3_256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha3_256(bytes).into()
    }
}

#[cfg(feature = "unstable")]
pub type Sha3_384 = DigestFn<Sha3_384Fn>;
#[cfg(feature = "unstable")]
pub struct Sha3_384Fn;

#[cfg(feature = "unstable")]
impl OutputSizeUser for Sha3_384Fn {
    type OutputSize = U48;
}

#[cfg(feature = "unstable")]
impl DigestFinalizer for Sha3_384Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha3_384(bytes).into()
    }
}

#[cfg(feature = "unstable")]
pub type Sha3_512 = DigestFn<Sha3_512Fn>;
#[cfg(feature = "unstable")]
pub struct Sha3_512Fn;

#[cfg(feature = "unstable")]
impl OutputSizeUser for Sha3_512Fn {
    type OutputSize = U64;
}

#[cfg(feature = "unstable")]
impl DigestFinalizer for Sha3_512Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha3_512(bytes).into()
    }
}
