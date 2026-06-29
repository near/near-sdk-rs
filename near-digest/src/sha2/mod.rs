use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Sha256 {
        near => self::near::Sha256,
        local => ::sha2::Sha256,
    }
}
