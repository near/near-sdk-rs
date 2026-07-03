//! SHA-3 hash family: Keccak and (unstable) SHA-3 variants

use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    /// Keccak-256 hasher
    ///
    /// Backed by the `keccak256_array` host function when compiled as a NEAR contract
    /// (`cfg(near)`), and by the pure-Rust implementation from the [`sha3`](https://docs.rs/sha3)
    /// crate otherwise.
    pub struct Keccak256 {
        near => self::near::Keccak256,
        _ => ::sha3::Keccak256,
    }
}

digest_cfg! {
    /// Keccak-512 hasher
    ///
    /// Backed by the `keccak512_array` host function when compiled as a NEAR contract
    /// (`cfg(near)`), and by the pure-Rust implementation from the [`sha3`](https://docs.rs/sha3)
    /// crate otherwise.
    pub struct Keccak512 {
        near => self::near::Keccak512,
        _ => ::sha3::Keccak512,
    }
}

#[cfg(feature = "unstable")]
digest_cfg! {
    /// Sha3-256 hasher
    ///
    /// There is currently no NEAR host function for SHA3, so this is computed in pure Rust on all
    /// targets, including on-chain. A host-function backend may be added in the future - hence the
    /// `unstable` feature gate.
    pub struct Sha3_256 {
        // TODO: Add `cfg(near)` path
        _ => ::sha3::Sha3_256
    }
}

#[cfg(feature = "unstable")]
digest_cfg! {
    /// Sha3-512 hasher
    ///
    /// There is currently no NEAR host function for SHA3, so this is computed in pure Rust on all
    /// targets, including on-chain. A host-function backend may be added in the future - hence the
    /// `unstable` feature gate.
    pub struct Sha3_512 {
        // TODO: Add `cfg(near)` path
        _ => ::sha3::Sha3_512
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
        hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
    )]
    #[case(
        b"near is cool!",
        hex!("5e7a77762908019844ab5c5432f9fc030ace77d2776b9535c03a9a184c9d3c59"),
    )]
    fn keccak_256_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 32]) {
        assert_eq!(Keccak256::digest(data), output, "hash has changed")
    }

    #[rstest]
    #[case(
        b"",
        hex!("0eab42de4c3ceb9235fc91acffe746b29c29a8c366b7c60e4e67c466f36a4304c00fa9caf9d87976ba469bcbe06713b435f091ef2769fb160cdab33d3670680e"),
    )]
    #[case(
        b"near is cool!",
        hex!("2e302ac2b8fca89a000940ff9264ffa2c46e3600bac574cddf3300b9b0d7cca7c974214a1b8ba2850ce894038c84bd835338b9673535da9fd13ab68ffd14df27"),
    )]
    fn keccak_512_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 64]) {
        assert_eq!(Keccak512::digest(data), output, "hash has changed")
    }

    #[cfg(feature = "unstable")]
    #[rstest]
    #[case(
        b"",
        hex!("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"),
    )]
    #[case(
        b"near is cool!",
        hex!("a839a0507ddbc3e7b23145a85bb11696e988cd771620b17de6be5063e8a721f7"),
    )]
    fn sha3_256_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 32]) {
        assert_eq!(Sha3_256::digest(data), output, "hash has changed")
    }

    #[cfg(feature = "unstable")]
    #[rstest]
    #[case(
        b"",
        hex!("a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26"),
    )]
    #[case(
        b"near is cool!",
        hex!("a787e4851d76e71c9ab4cd0061d31570a6511430faeffe5637a60340bbc94f2d1dde1080e4c2d6d22d01084b174bf214140b2dc5dae196e4741e6a42d1f49b96"),
    )]
    fn sha3_512_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 64]) {
        assert_eq!(Sha3_512::digest(data), output, "hash has changed")
    }
}
