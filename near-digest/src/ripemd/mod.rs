//! RIPEMD hash family.

use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    /// RIPEMD-160 hasher.
    ///
    /// Backed by the `ripemd160_array` host function when compiled as a NEAR contract
    /// (`cfg(near)`), and by the pure-Rust implementation from the
    /// [`ripemd`](https://docs.rs/ripemd) crate otherwise.
    pub struct Ripemd160 {
        near => self::near::Ripemd160,
        _ => ::ripemd::Ripemd160,
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
        hex!("9c1185a5c5e9fc54612808977ee8f548b2258d31"),
    )]
    #[case(
        b"near is cool!",
        hex!("320214cbbb6821fb23d3cd96dc0731ff5644323b"),
    )]
    fn ripemd160_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 20]) {
        assert_eq!(Ripemd160::digest(data), output, "hash has changed")
    }
}
