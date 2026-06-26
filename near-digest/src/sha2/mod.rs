use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Sha256 {
        near => self::near::Sha256,
        local => ::sha2::Sha256,
    }
}

digest_cfg! {
    pub struct Sha512 {
        near => self::near::Sha512,
        local => ::sha2::Sha512,
    }
}
