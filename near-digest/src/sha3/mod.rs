use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Keccak256 {
        near => self::near::Keccak256,
        _ => ::sha3::Keccak256,
    }
}

digest_cfg! {
    pub struct Keccak512 {
        near => self::near::Keccak512,
        _ => ::sha3::Keccak512,
    }
}

#[cfg(feature = "unstable")]
digest_cfg! {
    pub struct Sha3_256 {
        // TODO: Add `cfg(near)` path
        _ => ::sha3::Sha3_256
    }
}

#[cfg(feature = "unstable")]
digest_cfg! {
    pub struct Sha3_512 {
        // TODO: Add `cfg(near)` path
        _ => ::sha3::Sha3_512
    }
}
