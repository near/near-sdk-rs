use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Sha256 {
        near => self::near::Sha256,
        local => ::sha2::Sha256,
    }
}

#[cfg(feature = "unstable")]
digest_cfg! {
    pub struct Sha512 {
        // TODO: Add `cfg(near)` path
        local => ::sha2::Sha512,
    }
}
