use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Keccak256 {
        near => self::near::Keccak256,
        local => ::sha3::Keccak256,
    }
}

digest_cfg! {
    pub struct Keccak512 {
        near => self::near::Keccak512,
        local => ::sha3::Keccak512,
    }
}
