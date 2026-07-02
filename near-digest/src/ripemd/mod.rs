use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Ripemd160 {
        near => self::near::Ripemd160,
        _ => ::ripemd::Ripemd160,
    }
}
