//! SHA-2 hash family.

use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    /// SHA-256 hasher.
    ///
    /// Backed by the `sha256_array` host function when compiled as a NEAR contract (`cfg(near)`),
    /// and by the pure-Rust implementation from the [`sha2`](https://docs.rs/sha2) crate otherwise.
    pub struct Sha256 {
        near => self::near::Sha256,
        _ => ::sha2::Sha256,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use digest::Digest;
    use hex_literal::hex;
    use rstest::rstest;

    #[rstest]
    #[case(
        b"",
        hex!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    )]
    #[case(
        b"near is cool!",
        hex!("ba2b2a7c9d2c2c2505232a24f2b0e1b0c5781423957db7f8439b80a0292e9485"),
    )]
    fn sha256_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 32]) {
        assert!(Sha256::digest(data) == output, "has changed")
    }
}
