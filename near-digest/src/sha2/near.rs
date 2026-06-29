use digest::{Output, OutputSizeUser, consts::U32};

use crate::utils::{DigestFinalizer, DigestFn};

pub type Sha256 = DigestFn<Sha256Fn>;
pub struct Sha256Fn;

impl OutputSizeUser for Sha256Fn {
    type OutputSize = U32;
}

impl DigestFinalizer for Sha256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha256_array(bytes).into()
    }
}
