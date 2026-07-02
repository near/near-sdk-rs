use digest::{Output, OutputSizeUser, consts::U20};

use crate::utils::{DigestFinalizer, DigestFn};

pub type Ripemd160 = DigestFn<Ripemd160Fn>;
pub struct Ripemd160Fn;

impl OutputSizeUser for Ripemd160Fn {
    type OutputSize = U20;
}

impl DigestFinalizer for Ripemd160Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::ripemd160_array(bytes).into()
    }
}
